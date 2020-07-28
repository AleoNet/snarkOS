//! Implementation of public RPC endpoints.
//!
//! See [RpcFunctions](../trait.RpcFunctions.html) for documentation of public endpoints.

use crate::{rpc_trait::RpcFunctions, rpc_types::*};
use snarkos_consensus::{get_block_reward, ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkos_dpc::base_dpc::{
    instantiated::{Components, Tx},
    parameters::PublicParameters,
    record::DPCRecord,
};
use snarkos_errors::rpc::RpcError;
use snarkos_models::{dpc::Record, objects::Transaction};
use snarkos_network::{context::Context, process_transaction_internal};
use snarkos_objects::BlockHeaderHash;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
    CanonicalSerialize,
};

use chrono::Utc;
use std::sync::Arc;
use tokio::{runtime::Runtime, sync::Mutex};

/// Implements JSON-RPC HTTP endpoint functions for a node.
/// The constructor is given Arc::clone() copies of all needed node components.
#[derive(Clone)]
pub struct RpcImpl {
    /// Blockchain database storage.
    pub(crate) storage: Arc<MerkleTreeLedger>,

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
        parameters: PublicParameters<Components>,
        server_context: Arc<Context>,
        consensus: ConsensusParameters,
        memory_pool_lock: Arc<Mutex<MemoryPool<Tx>>>,
        credentials: Option<RpcCredentials>,
    ) -> Self {
        Self {
            storage,
            parameters,
            server_context,
            consensus,
            memory_pool_lock,
            credentials,
        }
    }
}

impl RpcFunctions for RpcImpl {
    /// Returns information about a block from a block hash.
    fn get_block(&self, block_hash_string: String) -> Result<BlockInfo, RpcError> {
        let block_hash = hex::decode(&block_hash_string)?;
        assert_eq!(block_hash.len(), 32);

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
        Ok(self.storage.get_block_count())
    }

    /// Returns the block hash of the head of the canonical chain.
    fn get_best_block_hash(&self) -> Result<String, RpcError> {
        let best_block_hash = self.storage.get_block_hash(self.storage.get_latest_block_height())?;

        Ok(hex::encode(&best_block_hash.0))
    }

    /// Returns the block hash of the index specified if it exists in the canonical chain.
    fn get_block_hash(&self, block_height: u32) -> Result<String, RpcError> {
        let block_hash = self.storage.get_block_hash(block_height)?;

        Ok(hex::encode(&block_hash.0))
    }

    /// Returns hex encoded bytes of a transaction from its transaction id.
    fn get_raw_transaction(&self, transaction_id: String) -> Result<String, RpcError> {
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
            digest: hex::encode(to_bytes![transaction.ledger_digest]?),
            transaction_proof: hex::encode(to_bytes![transaction.transaction_proof]?),
            predicate_commitment: hex::encode(to_bytes![transaction.predicate_commitment]?),
            local_data_commitment: hex::encode(to_bytes![transaction.local_data_commitment]?),
            value_balance: transaction.value_balance,
            signatures,
            transaction_metadata,
        })
    }

    /// Send raw transaction bytes to this node to be added into the mempool.
    /// If valid, the transaction will be stored and propagated to all peers.
    /// Returns the transaction id if valid.
    fn send_raw_transaction(&self, transaction_bytes: String) -> Result<String, RpcError> {
        let transaction_bytes = hex::decode(transaction_bytes)?;
        let transaction = Tx::read(&transaction_bytes[..])?;

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
        let block_height = self.storage.get_latest_block_height();
        let block = self.storage.get_block_from_block_number(block_height)?;

        let time = Utc::now().timestamp();

        let memory_pool = Runtime::new()?.block_on(self.memory_pool_lock.lock());
        let full_transactions = memory_pool.get_candidates(&self.storage, self.consensus.max_block_size)?;

        let transaction_strings = full_transactions.serialize_as_str()?;

        let coinbase_value = get_block_reward(block_height + 1) + full_transactions.calculate_transaction_fees();

        Ok(BlockTemplate {
            previous_block_hash: hex::encode(&block.header.get_hash().0),
            block_height: block_height + 1,
            time,
            difficulty_target: self.consensus.get_block_difficulty(&block.header, time),
            transactions: transaction_strings,
            coinbase_value,
        })
    }

    // Record handling

    /// Returns information about a record from serialized record bytes.
    fn decode_record(&self, record_bytes: String) -> Result<RecordInfo, RpcError> {
        let record_bytes = hex::decode(record_bytes)?;
        let record = DPCRecord::<Components>::read(&record_bytes[..])?;

        let owner = hex::encode(to_bytes![record.owner()]?);
        let payload = RPCRecordPayload {
            payload: hex::encode(to_bytes![record.payload()]?),
        };
        let birth_predicate_id = hex::encode(record.birth_predicate_id());
        let death_predicate_id = hex::encode(record.death_predicate_id());
        let serial_number_nonce = hex::encode(to_bytes![record.serial_number_nonce()]?);
        let commitment = hex::encode(to_bytes![record.commitment()]?);
        let commitment_randomness = hex::encode(to_bytes![record.commitment_randomness()]?);

        Ok(RecordInfo {
            owner,
            is_dummy: record.is_dummy(),
            value: record.value(),
            payload,
            birth_predicate_id,
            death_predicate_id,
            serial_number_nonce,
            commitment,
            commitment_randomness,
        })
    }
}
