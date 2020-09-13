// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

//! Implementation of public RPC endpoints.
//!
//! See [RpcFunctions](../trait.RpcFunctions.html) for documentation of public endpoints.

use crate::{rpc_trait::RpcFunctions, rpc_types::*};
use snarkos_consensus::{get_block_reward, ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkos_dpc::base_dpc::{
    instantiated::{Components, Tx},
    parameters::PublicParameters,
};
use snarkos_errors::rpc::RpcError;
use snarkos_models::objects::Transaction;
use snarkos_network::{context::Context, process_transaction_internal};
use snarkos_objects::BlockHeaderHash;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
    CanonicalSerialize,
};

use chrono::Utc;
use std::{path::PathBuf, sync::Arc};
use tokio::{runtime::Runtime, sync::Mutex};

/// Implements JSON-RPC HTTP endpoint functions for a node.
/// The constructor is given Arc::clone() copies of all needed node components.
#[derive(Clone)]
pub struct RpcImpl {
    /// Blockchain database storage.
    pub(crate) storage: Arc<MerkleTreeLedger>,

    /// The path to the Blockchain database storage.
    pub(crate) storage_path: PathBuf,

    /// Public Parameters
    pub(crate) parameters: PublicParameters<Components>,

    /// Network context held by the server.
    pub(crate) server_context: Arc<Context>,

    /// Consensus parameters generated from node config.
    pub(crate) consensus: ConsensusParameters,

    /// Handle to access the memory pool of transactions.
    pub(crate) memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,

    /// RPC credentials for accessing guarded endpoints
    pub(crate) credentials: Option<RpcCredentials>,
}

impl RpcImpl {
    /// Creates a new struct for calling public and private RPC endpoints.
    pub fn new(
        storage: Arc<MerkleTreeLedger>,
        storage_path: PathBuf,
        parameters: PublicParameters<Components>,
        server_context: Arc<Context>,
        consensus: ConsensusParameters,
        memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
        credentials: Option<RpcCredentials>,
    ) -> Self {
        Self {
            storage,
            storage_path,
            parameters,
            server_context,
            consensus,
            memory_pool_lock,
            credentials,
        }
    }

    /// Open a new secondary storage instance.
    pub fn new_secondary_storage_instance(&self) -> Result<MerkleTreeLedger, RpcError> {
        Ok(MerkleTreeLedger::open_secondary_at_path(self.storage_path.clone())?)
    }
}

impl RpcFunctions for RpcImpl {
    /// Returns information about a block from a block hash.
    fn get_block(&self, block_hash_string: String) -> Result<BlockInfo, RpcError> {
        let block_hash = hex::decode(&block_hash_string)?;
        assert_eq!(block_hash.len(), 32);

        self.storage.catch_up_secondary(false)?;

        let block_header_hash = BlockHeaderHash::new(block_hash);
        let height = match self.storage.get_block_number(&block_header_hash) {
            Ok(block_num) => match self.storage.is_canon(&block_header_hash) {
                true => Some(block_num),
                false => None,
            },
            Err(_) => None,
        };

        let confirmations = match height {
            Some(block_height) => self.storage.get_latest_block_height() - block_height,
            None => 0,
        };

        if let Ok(block) = self.storage.get_block(&block_header_hash) {
            let mut transactions = vec![];

            for transaction in block.transactions.iter() {
                transactions.push(hex::encode(&transaction.transaction_id()?));
            }

            Ok(BlockInfo {
                hash: block_hash_string,
                height,
                confirmations,
                size: block.serialize()?.len(),
                previous_block_hash: block.header.previous_block_hash.to_string(),
                merkle_root: block.header.merkle_root_hash.to_string(),
                pedersen_merkle_root_hash: block.header.pedersen_merkle_root_hash.to_string(),
                proof: block.header.proof.to_string(),
                time: block.header.time,
                difficulty_target: block.header.difficulty_target,
                nonce: block.header.nonce,
                transactions,
            })
        } else {
            Err(RpcError::InvalidBlockHash(block_hash_string))
        }
    }

    /// Returns the number of blocks in the canonical chain.
    fn get_block_count(&self) -> Result<u32, RpcError> {
        self.storage.catch_up_secondary(false)?;
        Ok(self.storage.get_block_count())
    }

    /// Returns the block hash of the head of the canonical chain.
    fn get_best_block_hash(&self) -> Result<String, RpcError> {
        self.storage.catch_up_secondary(false)?;
        let best_block_hash = self.storage.get_block_hash(self.storage.get_latest_block_height())?;

        Ok(hex::encode(&best_block_hash.0))
    }

    /// Returns the block hash of the index specified if it exists in the canonical chain.
    fn get_block_hash(&self, block_height: u32) -> Result<String, RpcError> {
        self.storage.catch_up_secondary(false)?;
        let block_hash = self.storage.get_block_hash(block_height)?;

        Ok(hex::encode(&block_hash.0))
    }

    /// Returns the hex encoded bytes of a transaction from its transaction id.
    fn get_raw_transaction(&self, transaction_id: String) -> Result<String, RpcError> {
        self.storage.catch_up_secondary(false)?;
        Ok(hex::encode(
            &self.storage.get_transaction_bytes(&hex::decode(transaction_id)?)?,
        ))
    }

    /// Returns information about a transaction from a transaction id.
    fn get_transaction_info(&self, transaction_id: String) -> Result<TransactionInfo, RpcError> {
        let transaction_bytes = self.get_raw_transaction(transaction_id)?;
        self.decode_raw_transaction(transaction_bytes)
    }

    /// Returns information about a transaction from serialized transaction bytes.
    fn decode_raw_transaction(&self, transaction_bytes: String) -> Result<TransactionInfo, RpcError> {
        self.storage.catch_up_secondary(false)?;
        let transaction_bytes = hex::decode(transaction_bytes)?;
        let transaction = Tx::read(&transaction_bytes[..])?;

        let mut old_serial_numbers = vec![];

        for sn in transaction.old_serial_numbers() {
            let mut serial_number: Vec<u8> = vec![];
            CanonicalSerialize::serialize(sn, &mut serial_number).unwrap();
            old_serial_numbers.push(hex::encode(serial_number));
        }

        let mut new_commitments = vec![];

        for cm in transaction.new_commitments() {
            new_commitments.push(hex::encode(to_bytes![cm]?));
        }

        let memo = hex::encode(to_bytes![transaction.memorandum()]?);

        let mut signatures = vec![];
        for sig in &transaction.signatures {
            signatures.push(hex::encode(to_bytes![sig]?));
        }

        let mut encrypted_records = vec![];

        for encrypted_record in &transaction.encrypted_records {
            encrypted_records.push(hex::encode(to_bytes![encrypted_record]?));
        }

        let transaction_id = transaction.transaction_id()?;
        let block_number = match self.storage.get_transaction_location(&transaction_id.to_vec())? {
            Some(block_location) => Some(
                self.storage
                    .get_block_number(&BlockHeaderHash(block_location.block_hash))?,
            ),
            None => None,
        };

        let transaction_metadata = TransactionMetadata { block_number };

        Ok(TransactionInfo {
            txid: hex::encode(&transaction_id),
            size: transaction_bytes.len(),
            old_serial_numbers,
            new_commitments,
            memo,
            network_id: transaction.network.id(),
            digest: hex::encode(to_bytes![transaction.ledger_digest]?),
            transaction_proof: hex::encode(to_bytes![transaction.transaction_proof]?),
            program_commitment: hex::encode(to_bytes![transaction.program_commitment]?),
            local_data_root: hex::encode(to_bytes![transaction.local_data_root]?),
            value_balance: transaction.value_balance.0,
            signatures,
            encrypted_records,
            transaction_metadata,
        })
    }

    /// Send raw transaction bytes to this node to be added into the mempool.
    /// If valid, the transaction will be stored and propagated to all peers.
    /// Returns the transaction id if valid.
    fn send_raw_transaction(&self, transaction_bytes: String) -> Result<String, RpcError> {
        let transaction_bytes = hex::decode(transaction_bytes)?;
        let transaction = Tx::read(&transaction_bytes[..])?;
        self.storage.catch_up_secondary(false)?;

        if !self
            .consensus
            .verify_transaction(&self.parameters, &transaction, &self.storage)?
        {
            // TODO (raychu86) Add more descriptive message. (e.g. tx already exists)
            return Ok("Transaction did not verify".into());
        }

        match !self.storage.transcation_conflicts(&transaction) {
            true => {
                Runtime::new()?.block_on(process_transaction_internal(
                    self.server_context.clone(),
                    &self.consensus,
                    &self.parameters,
                    self.storage.clone(),
                    self.memory_pool_lock.clone(),
                    to_bytes![transaction]?.to_vec(),
                    *Runtime::new()?.block_on(self.server_context.local_address.read()),
                ))?;

                Ok(hex::encode(transaction.transaction_id()?))
            }
            false => Ok("Transaction contains spent records".into()),
        }
    }

    /// Validate and return if the transaction is valid.
    fn validate_raw_transaction(&self, transaction_bytes: String) -> Result<bool, RpcError> {
        let transaction_bytes = hex::decode(transaction_bytes)?;
        let transaction = Tx::read(&transaction_bytes[..])?;
        self.storage.catch_up_secondary(false)?;

        Ok(self
            .consensus
            .verify_transaction(&self.parameters, &transaction, &self.storage)?)
    }

    /// Fetch the number of connected peers this node has.
    fn get_connection_count(&self) -> Result<usize, RpcError> {
        // Create a temporary tokio runtime to make an asynchronous function call
        let peer_book = Runtime::new()?.block_on(self.server_context.peer_book.read());

        Ok(peer_book.connected_total() as usize)
    }

    /// Returns this nodes connected peers.
    fn get_peer_info(&self) -> Result<PeerInfo, RpcError> {
        // Create a temporary tokio runtime to make an asynchronous function call
        let peer_book = Runtime::new()?.block_on(self.server_context.peer_book.read());

        let mut peers = vec![];

        for (peer, _last_seen) in &peer_book.get_connected() {
            peers.push(peer.clone());
        }

        Ok(PeerInfo { peers })
    }

    /// Returns the current mempool and consensus information known by this node.
    fn get_block_template(&self) -> Result<BlockTemplate, RpcError> {
        self.storage.catch_up_secondary(false)?;

        let block_height = self.storage.get_latest_block_height();
        let block = self.storage.get_block_from_block_number(block_height)?;

        let time = Utc::now().timestamp();

        let memory_pool = Runtime::new()?.block_on(self.memory_pool_lock.lock());
        let full_transactions = memory_pool.get_candidates(&self.storage, self.consensus.max_block_size)?;

        let transaction_strings = full_transactions.serialize_as_str()?;

        let mut coinbase_value = get_block_reward(block_height + 1);
        for transaction in full_transactions.iter() {
            coinbase_value = coinbase_value.add(transaction.value_balance())
        }

        Ok(BlockTemplate {
            previous_block_hash: hex::encode(&block.header.get_hash().0),
            block_height: block_height + 1,
            time,
            difficulty_target: self.consensus.get_block_difficulty(&block.header, time),
            transactions: transaction_strings,
            coinbase_value: coinbase_value.0 as u64,
        })
    }
}
