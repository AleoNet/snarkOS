use crate::{get_transaction_bytes, BlockStorage};
use snarkos_errors::storage::StorageError;
use snarkos_objects::{Block, BlockHeaderHash, Transactions};

impl BlockStorage {
    /// Get a block given the block hash
    pub fn get_block(&self, block_hash: &BlockHeaderHash) -> Result<Block, StorageError> {
        let block_transactions = self.get_block_transactions(block_hash)?;

        let mut transactions = vec![];
        for block_transaction_id in block_transactions {
            transactions.push(get_transaction_bytes(&self, &block_transaction_id).unwrap());
        }

        Ok(Block {
            header: self.get_block_header(&block_hash)?,
            transactions: Transactions::from(&transactions),
        })
    }

    /// Get a block given the block number
    pub fn get_block_from_block_num(&self, block_num: u32) -> Result<Block, StorageError> {
        if block_num > self.get_latest_block_height() {
            return Err(StorageError::InvalidBlockNumber(block_num));
        }

        let block_hash = self.get_block_hash(block_num)?;

        self.get_block(&block_hash)
    }

    /// Returns the block number of a conflicting block that has already been mined
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
}
