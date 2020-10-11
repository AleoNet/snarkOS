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

use crate::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkos_dpc::base_dpc::{instantiated::*, parameters::PublicParameters, record::DPCRecord};
use snarkos_errors::consensus::ConsensusError;
use snarkos_models::{
    algorithms::{LoadableMerkleParameters, CRH},
    dpc::{DPCScheme, Record},
    objects::Transaction,
};
use snarkos_objects::{dpc::DPCTransactions, AccountAddress, Block, BlockHeader};
use snarkos_posw::{txids_to_roots, PoswMarlin};
use snarkos_storage::Ledger;
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use chrono::Utc;
use rand::{thread_rng, Rng};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Compiles transactions into blocks to be submitted to the network.
/// Uses a proof of work based algorithm to find valid blocks.
#[derive(Clone)]
pub struct Miner {
    /// The coinbase address that mining rewards are assigned to.
    address: AccountAddress<Components>,
    /// The consensus parameters for the network of this miner.
    pub consensus_parameters: ConsensusParameters,
    /// The mining instance that is initialized with a proving key.
    miner: PoswMarlin,
}

impl Miner {
    /// Creates a new instance of `Miner`.
    pub fn new(address: AccountAddress<Components>, consensus_parameters: ConsensusParameters) -> Self {
        Self {
            address,
            consensus_parameters,
            // Load the miner with the proving key, this should never fail
            miner: PoswMarlin::load().expect("could not instantiate the miner"),
        }
    }

    /// Fetches new transactions from the memory pool.
    pub async fn fetch_memory_pool_transactions<T: Transaction, P: LoadableMerkleParameters>(
        storage: &Arc<RwLock<Ledger<T, P>>>,
        memory_pool: &Arc<Mutex<MemoryPool<T>>>,
        max_size: usize,
    ) -> Result<DPCTransactions<T>, ConsensusError> {
        let memory_pool = memory_pool.lock().await;
        Ok(memory_pool.get_candidates(&*storage.read().await, max_size)?)
    }

    /// Add a coinbase transaction to a list of candidate block transactions
    pub fn add_coinbase_transaction<R: Rng>(
        &self,
        parameters: &PublicParameters<Components>,
        storage: &MerkleTreeLedger,
        transactions: &mut DPCTransactions<Tx>,
        rng: &mut R,
    ) -> Result<Vec<DPCRecord<Components>>, ConsensusError> {
        let program_vk_hash = to_bytes![ProgramVerificationKeyCRH::hash(
            &parameters.system_parameters.program_verification_key_crh,
            &to_bytes![parameters.noop_program_snark_parameters.verification_key]?
        )?]?;

        let new_birth_programs = vec![program_vk_hash.clone(); NUM_OUTPUT_RECORDS];
        let new_death_programs = vec![program_vk_hash.clone(); NUM_OUTPUT_RECORDS];

        for transaction in transactions.iter() {
            if self.consensus_parameters.network_id != transaction.network {
                return Err(ConsensusError::ConflictingNetworkId(
                    self.consensus_parameters.network_id.id(),
                    transaction.network.id(),
                ));
            }
        }

        let (records, tx) = self.consensus_parameters.create_coinbase_transaction(
            storage.get_current_block_height() + 1,
            transactions,
            parameters,
            &program_vk_hash,
            new_birth_programs,
            new_death_programs,
            self.address.clone(),
            &storage,
            rng,
        )?;

        transactions.push(tx);
        Ok(records)
    }

    /// Acquires the storage lock and returns the previous block header and verified transactions.
    pub fn establish_block(
        &self,
        parameters: &PublicParameters<Components>,
        storage: &MerkleTreeLedger,
        transactions: &DPCTransactions<Tx>,
    ) -> Result<(BlockHeader, DPCTransactions<Tx>, Vec<DPCRecord<Components>>), ConsensusError> {
        let rng = &mut thread_rng();
        let mut transactions = transactions.clone();
        let coinbase_records = self.add_coinbase_transaction(parameters, &storage, &mut transactions, rng)?;

        // Verify transactions
        assert!(InstantiatedDPC::verify_transactions(
            parameters,
            &transactions.0,
            storage
        )?);

        let previous_block_header = storage.get_latest_block()?.header;

        Ok((previous_block_header, transactions, coinbase_records))
    }

    /// Run proof of work to find block.
    /// Returns BlockHeader with nonce solution.
    pub fn find_block<T: Transaction>(
        &self,
        transactions: &DPCTransactions<T>,
        parent_header: &BlockHeader,
    ) -> Result<BlockHeader, ConsensusError> {
        let txids = transactions.to_transaction_ids()?;
        let (merkle_root_hash, pedersen_merkle_root_hash, subroots) = txids_to_roots(&txids);

        let time = Utc::now().timestamp();
        let difficulty_target = self.consensus_parameters.get_block_difficulty(parent_header, time);

        // TODO: Switch this to use a user-provided RNG
        let (nonce, proof) = self.miner.mine(
            &subroots,
            difficulty_target,
            &mut thread_rng(),
            self.consensus_parameters.max_nonce,
        )?;

        Ok(BlockHeader {
            previous_block_hash: parent_header.get_hash(),
            merkle_root_hash,
            pedersen_merkle_root_hash,
            time,
            difficulty_target,
            nonce,
            proof: proof.into(),
        })
    }

    /// Returns a mined block.
    /// Calls methods to fetch transactions, run proof of work, and add the block into the chain for storage.
    pub async fn mine_block(
        &self,
        parameters: &PublicParameters<Components>,
        storage: &Arc<RwLock<MerkleTreeLedger>>,
        memory_pool: &Arc<Mutex<MemoryPool<Tx>>>,
    ) -> Result<(Vec<u8>, Vec<DPCRecord<Components>>), ConsensusError> {
        let mut candidate_transactions = Self::fetch_memory_pool_transactions(
            &storage.clone(),
            memory_pool,
            self.consensus_parameters.max_block_size,
        )
        .await?;

        println!("Miner creating block");

        let (previous_block_header, transactions, coinbase_records) =
            self.establish_block(parameters, &*storage.read().await, &mut candidate_transactions)?;

        println!("Miner generated coinbase transaction");

        for (index, record) in coinbase_records.iter().enumerate() {
            let record_commitment = hex::encode(&to_bytes![record.commitment()]?);
            println!("Coinbase record {:?} commitment: {:?}", index, record_commitment);
        }

        let header = self.find_block(&transactions, &previous_block_header)?;

        println!("Miner found block");

        let block = Block { header, transactions };

        let mut memory_pool = memory_pool.lock().await;

        self.consensus_parameters
            .receive_block(parameters, &*storage.read().await, &mut memory_pool, &block)?;

        // Store the non-dummy coinbase records.
        let mut records_to_store = vec![];
        for record in &coinbase_records {
            if !record.is_dummy() {
                records_to_store.push(record.clone());
            }
        }
        &storage.read().await.store_records(&records_to_store)?;

        Ok((block.serialize()?, coinbase_records))
    }
}
