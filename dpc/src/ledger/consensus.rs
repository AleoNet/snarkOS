use snarkos_errors::consensus::ConsensusError;
use snarkos_objects::{merkle_root, Block, BlockHeader, BlockHeaderHash, MerkleRootHash, Transactions};

use std::time::{SystemTime, UNIX_EPOCH};

pub const TWO_HOURS_UNIX: i64 = 7200;

/// Parameters for a proof of work blockchain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsensusParameters {
    /// Maximum block size in bytes
    pub max_block_size: usize,

    /// Maximum nonce value allowed
    pub max_nonce: u32,

    /// The amount of time it should take to find a block
    pub target_block_time: i64,

    /// Maximum transaction size in bytes
    pub transaction_size: usize,
    // /// mainnet or testnet
    //    network: Network
}

/// Calculate a block reward that halves every 1000 blocks.
pub fn block_reward(block_num: u32) -> u64 {
    100_000_000u64 / (2_u64.pow(block_num / 1000))
}

impl ConsensusParameters {
    /// Calculate the difficulty for the next block based off how long it took to mine the last one.
    pub fn get_block_difficulty(&self, prev_header: &BlockHeader, _block_timestamp: i64) -> u64 {
        //        bitcoin_retarget(
        //            block_timestamp,
        //            prev_header.time,
        //            self.target_block_time,
        //            prev_header.difficulty_target,
        //        )

        prev_header.difficulty_target
    }

    pub fn is_genesis(block: &Block) -> bool {
        block.header.previous_block_hash == BlockHeaderHash([0u8; 32])
    }

    /// Verify all fields in a block header.
    /// 1. The parent hash points to the tip of the chain.
    /// 2. Transactions hash to merkle root.
    /// 3. The timestamp is less than 2 hours into the future.
    /// 4. The timestamp is greater than parent timestamp.
    /// 5. The header is greater than or equal to target difficulty.
    /// 6. The nonce is within the limit.
    pub fn verify_header(
        &self,
        header: &BlockHeader,
        parent_header: &BlockHeader,
        merkle_root_hash: &MerkleRootHash,
    ) -> Result<(), ConsensusError> {
        let hash_result = header.to_difficulty_hash();

        let since_the_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let future_timelimit: i64 = since_the_epoch as i64 + TWO_HOURS_UNIX;

        if parent_header.get_hash() != header.previous_block_hash {
            Err(ConsensusError::NoParent(
                parent_header.get_hash().to_string(),
                header.previous_block_hash.to_string(),
            ))
        } else if header.merkle_root_hash != *merkle_root_hash {
            Err(ConsensusError::MerkleRoot(header.merkle_root_hash.to_string()))
        } else if header.time > future_timelimit {
            Err(ConsensusError::FuturisticTimestamp(future_timelimit, header.time))
        } else if header.time < parent_header.time {
            Err(ConsensusError::TimestampInvalid(header.time, parent_header.time))
        } else if hash_result > header.difficulty_target {
            Err(ConsensusError::PowInvalid(header.difficulty_target, hash_result))
        } else if header.nonce >= self.max_nonce {
            Err(ConsensusError::NonceInvalid(header.nonce, self.max_nonce))
        } else {
            Ok(())
        }
    }

    /// Verify that a block's transactions are valid.
    /// Check all outpoints, verify signatures, and calculate transaction fees.
    pub fn verify_transactions(
        &self,
        //        storage: &BlockStorage,
        _transactions: &Transactions,
    ) -> Result<(), ConsensusError> {
        Ok(())
    }

    /// Verifies that the block header is valid.
    pub fn valid_block_header(&self, block: &Block) -> Result<(), ConsensusError> {
        let mut merkle_root_slice = [0u8; 32];
        merkle_root_slice.copy_from_slice(&merkle_root(&block.transactions.to_transaction_ids()?));
        let _merkle_root_hash = &MerkleRootHash(merkle_root_slice);

        // Do not verify headers of genesis blocks
        //        if !Self::is_genesis(block) {
        //            let parent_block = storage.get_latest_block()?;
        //            self.verify_header(&block.header, &parent_block.header, merkle_root_hash)?;
        //        }

        Ok(())
    }

    /// Return whether or not the given block is valid and insert it.
    /// 1. Verify that the block header is valid.
    /// 2. Verify that the transactions are valid.
    /// 3. Insert/canonize block.
    /// 4. Check cached blocks to insert/canonize.
    pub fn process_block(
        &self,
        //        storage: &BlockStorage,
        //        memory_pool: &mut MemoryPool,
        _block: &Block,
    ) -> Result<u32, ConsensusError> {
        Ok(0)
    }
}
