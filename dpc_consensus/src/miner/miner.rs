use crate::{miner::MemoryPool, ConsensusParameters};

use snarkos_algorithms::merkle_tree::MerkleParameters;
use snarkos_dpc::{
    address::AddressPublicKey,
    base_dpc::{instantiated::*, parameters::PublicParameters},
    DPCScheme,
};
use snarkos_dpc_storage::BlockStorage;

use snarkos_errors::consensus::ConsensusError;
use snarkos_objects::{
    dpc::{Block, DPCTransactions, Transaction},
    merkle_root,
    BlockHeader,
    MerkleRootHash,
};
use snarkos_utilities::bytes::FromBytes;

use chrono::Utc;
use rand::{thread_rng, Rng};
use snarkos_dpc::dpc::base_dpc::record::DPCRecord;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Compiles transactions into blocks to be submitted to the network.
/// Uses a proof of work based algorithm to find valid blocks.
#[derive(Clone)]
pub struct Miner {
    /// Receiving address that block rewards will be sent to.
    address: AddressPublicKey<Components>,

    /// Parameters for current blockchain consensus.
    pub consensus: ConsensusParameters,
}

impl Miner {
    /// Returns a new instance of a miner with consensus params.
    pub fn new(address: AddressPublicKey<Components>, consensus: ConsensusParameters) -> Self {
        Self { address, consensus }
    }

    /// Fetches new transactions from the memory pool.
    pub async fn fetch_memory_pool_transactions<T: Transaction, P: MerkleParameters>(
        storage: &Arc<BlockStorage<T, P>>,
        memory_pool: &Arc<Mutex<MemoryPool<T>>>,
        max_size: usize,
    ) -> Result<DPCTransactions<T>, ConsensusError> {
        let memory_pool = memory_pool.lock().await;
        Ok(memory_pool.get_candidates(&storage, max_size)?)
    }

    pub fn add_coinbase_transaction<R: Rng>(
        &self,
        parameters: &PublicParameters<Components>,
        storage: &MerkleTreeLedger,
        transactions: &mut DPCTransactions<Tx>,
        rng: &mut R,
    ) -> Result<Vec<DPCRecord<Components>>, ConsensusError> {
        let genesis_pred_vk_bytes = storage.genesis_pred_vk_bytes()?;
        let genesis_address_pair = FromBytes::read(&storage.genesis_address_pair_bytes()?[..])?;

        let new_predicate = Predicate::new(genesis_pred_vk_bytes.clone());
        let new_birth_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];
        let new_death_predicates = vec![new_predicate.clone(); NUM_OUTPUT_RECORDS];

        let (records, tx) = ConsensusParameters::create_coinbase_transaction(
            storage.get_latest_block_height() + 1,
            transactions,
            parameters,
            &genesis_pred_vk_bytes,
            new_birth_predicates,
            new_death_predicates,
            genesis_address_pair,
            self.address.clone(),
            &storage,
            rng,
        )?;

        transactions.push(tx);
        Ok(records)
    }

    /// Acquires the storage lock and returns the previous block header and verified transactions.
    pub async fn establish_block(
        &self,
        parameters: &PublicParameters<Components>,
        storage: &MerkleTreeLedger,
        transactions: &DPCTransactions<Tx>,
    ) -> Result<(BlockHeader, DPCTransactions<Tx>, Vec<DPCRecord<Components>>), ConsensusError> {
        let rng = &mut thread_rng();
        let mut transactions = transactions.clone();
        let coinbase_records = self.add_coinbase_transaction(parameters, &storage, &mut transactions, rng)?;

        // Verify transactions
        InstantiatedDPC::verify_transactions(parameters, &transactions.0, storage)?;

        let previous_block_header = storage.get_latest_block()?.header;

        Ok((previous_block_header, transactions, coinbase_records))
    }

    /// Run proof of work to find block.
    /// Returns BlockHeader with nonce solution.
    pub fn find_block(
        &self,
        transactions: &DPCTransactions<Tx>,
        parent_header: &BlockHeader,
    ) -> Result<BlockHeader, ConsensusError> {
        let mut merkle_root_bytes = [0u8; 32];
        merkle_root_bytes[..].copy_from_slice(&merkle_root(&transactions.to_transaction_ids()?));

        let time = Utc::now().timestamp();

        let header = BlockHeader {
            merkle_root_hash: MerkleRootHash(merkle_root_bytes),
            previous_block_hash: parent_header.get_hash(),
            time,
            difficulty_target: self.consensus.get_block_difficulty(parent_header, time),
            nonce: 0u32,
        };

        let mut hash_input = header.serialize();

        loop {
            let nonce = rand::thread_rng().gen_range(0, self.consensus.max_nonce);

            hash_input[80..84].copy_from_slice(&nonce.to_le_bytes());
            let hash_result = BlockHeader::deserialize(&hash_input).to_difficulty_hash();

            if hash_result <= header.difficulty_target {
                return Ok(BlockHeader::deserialize(&hash_input));
            }
        }
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

        let (previous_block_header, transactions, coinbase_records) = self
            .establish_block(parameters, storage, &mut candidate_transactions)
            .await?;

        let header = self.find_block(&transactions, &previous_block_header)?;

        let block = Block { header, transactions };

        let mut memory_pool = memory_pool.lock().await;

        self.consensus
            .receive_block(parameters, storage, &mut memory_pool, &block)?;

        Ok((block.serialize()?, coinbase_records))
    }
}
