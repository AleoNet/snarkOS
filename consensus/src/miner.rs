use crate::{ConsensusParameters, MemoryPool, MerkleTreeLedger};
use snarkos_dpc::{
    base_dpc::{instantiated::*, parameters::PublicParameters},
    dpc::base_dpc::record::DPCRecord,
};
use snarkos_errors::consensus::ConsensusError;
use snarkos_models::{
    algorithms::{MerkleParameters, CRH},
    dpc::{DPCScheme, Record},
    objects::Transaction,
};
use snarkos_objects::{dpc::DPCTransactions, AccountPublicKey, Block, BlockHeader};
use snarkos_posw::{txids_to_roots, Posw};
use snarkos_storage::Ledger;
use snarkos_utilities::{bytes::ToBytes, to_bytes};

use chrono::Utc;
use rand::{thread_rng, Rng};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Compiles transactions into blocks to be submitted to the network.
/// Uses a proof of work based algorithm to find valid blocks.
#[derive(Clone)]
pub struct Miner {
    /// Receiving address that block rewards will be sent to.
    address: AccountPublicKey<Components>,

    /// Parameters for current blockchain consensus.
    pub consensus: ConsensusParameters,

    /// The miner instance (must be initialized with a Proving Key)
    miner: Posw,
}

impl Miner {
    /// Returns a new instance of a miner with consensus params.
    pub fn new(address: AccountPublicKey<Components>, consensus: ConsensusParameters) -> Self {
        Self {
            address,
            consensus,
            // load the miner with the proving key, this should never fail
            miner: Posw::load().expect("could not instantiate the miner"),
        }
    }

    /// Fetches new transactions from the memory pool.
    pub async fn fetch_memory_pool_transactions<T: Transaction, P: MerkleParameters>(
        storage: &Arc<Ledger<T, P>>,
        memory_pool: &Arc<Mutex<MemoryPool<T>>>,
        max_size: usize,
    ) -> Result<DPCTransactions<T>, ConsensusError> {
        let memory_pool = memory_pool.lock().await;
        Ok(memory_pool.get_candidates(&storage, max_size)?)
    }

    /// Add a coinbase transaction to a list of candidate block transactions
    pub fn add_coinbase_transaction<R: Rng>(
        &self,
        parameters: &PublicParameters<Components>,
        storage: &MerkleTreeLedger,
        transactions: &mut DPCTransactions<Tx>,
        rng: &mut R,
    ) -> Result<Vec<DPCRecord<Components>>, ConsensusError> {
        let predicate_vk_hash = to_bytes![PredicateVerificationKeyHash::hash(
            &parameters.circuit_parameters.predicate_verification_key_hash,
            &to_bytes![parameters.predicate_snark_parameters.verification_key]?
        )?]?;

        let new_predicate = Predicate::new(predicate_vk_hash.clone());
        let new_birth_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];
        let new_death_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];

        for transaction in transactions.iter() {
            if self.consensus.network_id != transaction.network_id {
                return Err(ConsensusError::ConflictingNetworkId(
                    self.consensus.network_id,
                    transaction.network_id,
                ));
            }
        }

        let (records, tx) = self.consensus.create_coinbase_transaction(
            storage.get_latest_block_height() + 1,
            transactions,
            parameters,
            &predicate_vk_hash,
            new_birth_predicates,
            new_death_predicates,
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
        let difficulty_target = self.consensus.get_block_difficulty(parent_header, time);

        // TODO: Switch this to use a user-provided RNG
        let (nonce, proof) = self
            .miner
            .mine(
                &subroots,
                difficulty_target,
                &mut thread_rng(),
                self.consensus.max_nonce,
            )
            .unwrap();

        Ok(BlockHeader {
            previous_block_hash: parent_header.get_hash(),
            merkle_root_hash,
            pedersen_merkle_root_hash,
            time,
            difficulty_target,
            nonce,
            proof,
        })
    }

    /// Returns a mined block.
    /// Calls methods to fetch transactions, run proof of work, and add the block into the chain for storage.
    pub async fn mine_block(
        &self,
        parameters: &PublicParameters<Components>,
        storage: &Arc<MerkleTreeLedger>,
        memory_pool: &Arc<Mutex<MemoryPool<Tx>>>,
    ) -> Result<(Vec<u8>, Vec<DPCRecord<Components>>), ConsensusError> {
        let mut candidate_transactions =
            Self::fetch_memory_pool_transactions(&storage.clone(), memory_pool, self.consensus.max_block_size).await?;

        println!("Miner creating block");

        let (previous_block_header, transactions, coinbase_records) =
            self.establish_block(parameters, storage, &mut candidate_transactions)?;

        println!("Miner generated coinbase transaction");

        for (index, record) in coinbase_records.iter().enumerate() {
            let record_commitment = hex::encode(&to_bytes![record.commitment()]?);
            println!("Coinbase record {:?} commitment: {:?}", index, record_commitment);
        }

        let header = self.find_block(&transactions, &previous_block_header)?;

        println!("Miner found block block");

        let block = Block { header, transactions };

        let mut memory_pool = memory_pool.lock().await;

        self.consensus
            .receive_block(parameters, storage, &mut memory_pool, &block)?;

        storage.store_records(&coinbase_records)?;

        Ok((block.serialize()?, coinbase_records))
    }
}
