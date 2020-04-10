use crate::{BlockStorage, KEY_BEST_BLOCK_NUMBER};
use snarkos_errors::{objects::BlockError, storage::StorageError};
use snarkos_objects::{Block, BlockHeader, BlockHeaderHash, Transactions};

use std::collections::HashMap;

impl BlockStorage {
    /// Get a block given the block hash.
    pub fn get_block(&self, block_hash: &BlockHeaderHash) -> Result<Block, StorageError> {
        let block_transactions = self.get_block_transactions(block_hash)?;

        let mut transactions = vec![];
        for block_transaction_id in block_transactions {
            transactions.push(self.get_transaction_bytes(&block_transaction_id)?);
        }

        Ok(Block {
            header: self.get_block_header(&block_hash)?,
            transactions: Transactions::from(&transactions),
        })
    }

    /// Get a block given the block number.
    pub fn get_block_from_block_num(&self, block_num: u32) -> Result<Block, StorageError> {
        if block_num > self.get_latest_block_height() {
            return Err(StorageError::BlockError(BlockError::InvalidBlockNumber(block_num)));
        }

        let block_hash = self.get_block_hash(block_num)?;

        self.get_block(&block_hash)
    }

    /// Get the latest block in the chain.
    pub fn get_latest_block(&self) -> Result<Block, StorageError> {
        self.get_block_from_block_num(self.get_latest_block_height())
    }

    /// Returns true if there are no blocks in the chain.
    pub fn is_empty(&self) -> bool {
        self.get_latest_block().is_err()
    }

    /// Find the potential parent block given a block header.
    pub fn find_parent_block(&self, block_header: &BlockHeader) -> Result<Block, StorageError> {
        self.get_block(&block_header.previous_block_hash)
    }

    /// Returns the block number of a conflicting block that has already been mined.
    pub fn already_mined(&self, block: &Block) -> Result<Option<u32>, StorageError> {
        // look up new block's previous block by hash
        // if the block after previous_block_number exists, then someone has already mined this new block
        let previous_block_number = self.get_block_num(&block.header.previous_block_hash)?;

        let existing_block_number = previous_block_number + 1;

        if self.get_block_from_block_num(existing_block_number).is_ok() {
            // the storage has a conflicting block with the same previous_block_hash
            Ok(Some(existing_block_number))
        } else {
            // the new block has no conflicts
            Ok(None)
        }
    }

    /// Remove a block and it's related data from the storage.
    pub fn remove_block(&self, block_hash: BlockHeaderHash) -> Result<(), StorageError> {
        unimplemented!()
    }

    /// Remove the latest block.
    pub fn remove_latest_block(&self) -> Result<(), StorageError> {
        unimplemented!()
    }

    /// Remove the latest `num_blocks` blocks.
    pub fn remove_latest_blocks(&self, num_blocks: u32) -> Result<(), StorageError> {
        let latest_block_height = self.get_latest_block_height();
        if num_blocks > latest_block_height {
            return Err(StorageError::InvalidBlockRemovalNum(num_blocks, latest_block_height));
        }

        for _ in 0..num_blocks {
            self.remove_latest_block()?;
        }
        Ok(())
    }
}
