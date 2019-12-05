use crate::{block_reward, check_block_transactions, miner::MemoryPool, ConsensusParameters};
use snarkos_errors::consensus::ConsensusError;
use snarkos_objects::{merkle_root, Block, BlockHeader, MerkleRootHash, Transaction, Transactions};
use snarkos_storage::BlockStorage;

use chrono::Utc;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::Mutex;
use wagyu_bitcoin::{BitcoinAddress, Mainnet};

#[derive(Clone)]
pub struct Miner {
    // receiving address that block rewards will be sent to
    address: BitcoinAddress<Mainnet>,

    // parameters for current blockchain consensus
    pub consensus: ConsensusParameters,
}

impl Miner {
    /// Returns a new instance of a miner with consensus params
    pub fn new(address: BitcoinAddress<Mainnet>, consensus: ConsensusParameters) -> Self {
        Self { address, consensus }
    }

    /// Fetches new transactions from the memory pool
    pub async fn fetch_memory_pool_transactions(
        storage: &Arc<BlockStorage>,
        memory_pool: &Arc<Mutex<MemoryPool>>,
        max_size: usize,
    ) -> Result<Transactions, ConsensusError> {
        let memory_pool = memory_pool.lock().await;
        Ok(memory_pool.get_candidates(&storage, max_size)?)
    }

    pub fn add_coinbase_transaction(
        &self,
        storage: &BlockStorage,
        transactions: &mut Transactions,
    ) -> Result<(), ConsensusError> {
        let transaction_fees = storage.calculate_transaction_fees(&transactions)?;
        transactions.insert(
            0,
            Transaction::create_coinbase_transaction(
                storage.get_latest_block_height() + 1,
                block_reward(storage.get_latest_block_height() + 1),
                transaction_fees,
                &self.address,
            )?,
        );
        Ok(())
    }

    /// Acquires the storage lock and returns the previous block header and verified transactions
    pub async fn establish_block(
        &self,
        storage: &Arc<BlockStorage>,
        transactions: &Transactions,
    ) -> Result<(BlockHeader, Transactions), ConsensusError> {
        let mut transactions = transactions.clone();
        self.add_coinbase_transaction(&storage, &mut transactions)?;
        check_block_transactions(&storage, &transactions)?;

        let previous_block_header = storage.get_latest_block()?.header;

        Ok((previous_block_header, transactions))
    }

    /// Run proof of work to find block. Returns BlockHeader with nonce solution
    pub fn find_block(
        &self,
        transactions: &Transactions,
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

    /// Mines the next block
    pub async fn mine_block(
        &self,
        storage: &Arc<BlockStorage>,
        memory_pool: &Arc<Mutex<MemoryPool>>,
    ) -> Result<Vec<u8>, ConsensusError> {
        let mut candidate_transactions =
            Self::fetch_memory_pool_transactions(&storage.clone(), memory_pool, self.consensus.max_block_size).await?;

        let (previous_block_header, transactions) = self.establish_block(storage, &mut candidate_transactions).await?;

        let header = self.find_block(&transactions, &previous_block_header)?;

        let block = Block { header, transactions };

        let mut memory_pool = memory_pool.lock().await;

        self.consensus.receive_block(storage, &mut memory_pool, &block)?;

        Ok(block.serialize()?)
    }
}
