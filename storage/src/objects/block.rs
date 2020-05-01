use crate::*;
use snarkos_algorithms::merkle_tree::MerkleParameters;
use snarkos_errors::{objects::BlockError, storage::StorageError};
use snarkos_objects::{
    dpc::{Block, Transaction},
    BlockHeader,
    BlockHeaderHash,
};

use snarkos_utilities::{bytes::ToBytes, to_bytes};

//use std::collections::HashMap;

impl<T: Transaction, P: MerkleParameters> BlockStorage<T, P> {
    /// Get a block given the block hash.
    pub fn get_block(&self, block_hash: &BlockHeaderHash) -> Result<Block<T>, StorageError> {
        Ok(Block {
            header: self.get_block_header(block_hash)?,
            transactions: self.get_block_transactions(block_hash)?,
        })
    }

    /// Get a block given the block number.
    pub fn get_block_from_block_num(&self, block_num: u32) -> Result<Block<T>, StorageError> {
        if block_num > self.get_latest_block_height() {
            return Err(StorageError::BlockError(BlockError::InvalidBlockNumber(block_num)));
        }

        let block_hash = self.get_block_hash(block_num)?;

        self.get_block(&block_hash)
    }

    /// Get the latest block in the chain.
    pub fn get_latest_block(&self) -> Result<Block<T>, StorageError> {
        self.get_block_from_block_num(self.get_latest_block_height())
    }

    /// Returns true if there are no blocks in the chain.
    pub fn is_empty(&self) -> bool {
        self.get_latest_block().is_err()
    }

    /// Find the potential parent block given a block header.
    pub fn find_parent_block(&self, block_header: &BlockHeader) -> Result<Block<T>, StorageError> {
        self.get_block(&block_header.previous_block_hash)
    }

    /// Returns the block number of a conflicting block that has already been mined.
    pub fn already_mined(&self, block: &Block<T>) -> Result<Option<u32>, StorageError> {
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
    pub fn remove_block(&self, block_hash: BlockHeaderHash) -> Result<DatabaseTransaction, StorageError> {
        let latest_block_height = self.get_latest_block_height();
        if latest_block_height == 0 {
            return Err(StorageError::InvalidBlockRemovalNum(0, 0));
        }

        let block_transactions = self.get_block_transactions(&block_hash)?;

        let mut sn_index = self.current_sn_index()?;
        let mut cm_index = self.current_cm_index()?;
        let mut memo_index = self.current_memo_index()?;

        let mut database_transaction = DatabaseTransaction::new();

        if let Ok(block_num) = self.get_block_num(&block_hash) {
            database_transaction.push(Op::Delete {
                col: COL_BLOCK_LOCATOR,
                key: block_num.to_le_bytes().to_vec(),
            });
        };

        database_transaction.push(Op::Delete {
            col: COL_BLOCK_TRANSACTIONS,
            key: block_hash.0.to_vec(),
        });

        database_transaction.push(Op::Delete {
            col: COL_BLOCK_LOCATOR,
            key: block_hash.0.to_vec(),
        });

        for transaction in block_transactions.0 {
            database_transaction.push(Op::Delete {
                col: COL_TRANSACTION_LOCATION,
                key: transaction.transaction_id()?.to_vec(),
            });

            for sn in transaction.old_serial_numbers() {
                database_transaction.push(Op::Delete {
                    col: COL_SERIAL_NUMBER,
                    key: to_bytes![sn]?.to_vec(),
                });
                sn_index -= 1;
            }

            for cm in transaction.new_commitments() {
                database_transaction.push(Op::Delete {
                    col: COL_COMMITMENT,
                    key: to_bytes![cm]?.to_vec(),
                });
                cm_index -= 1;
            }

            database_transaction.push(Op::Delete {
                col: COL_MEMO,
                key: to_bytes![transaction.memorandum()]?.to_vec(),
            });
            memo_index -= 1;
        }

        // Update the database state for current indexes

        database_transaction.push(Op::Insert {
            col: COL_META,
            key: KEY_CURR_SN_INDEX.as_bytes().to_vec(),
            value: (sn_index as u32).to_le_bytes().to_vec(),
        });
        database_transaction.push(Op::Insert {
            col: COL_META,
            key: KEY_CURR_CM_INDEX.as_bytes().to_vec(),
            value: (cm_index as u32).to_le_bytes().to_vec(),
        });
        database_transaction.push(Op::Insert {
            col: COL_META,
            key: KEY_CURR_MEMO_INDEX.as_bytes().to_vec(),
            value: (memo_index as u32).to_le_bytes().to_vec(),
        });

        Ok(database_transaction)
    }

    /// Remove the latest block.
    pub fn remove_latest_block(&self) -> Result<(), StorageError> {
        let latest_block_height = self.get_latest_block_height();
        if latest_block_height == 0 {
            return Err(StorageError::InvalidBlockRemovalNum(0, 0));
        }

        let update_best_block_num = latest_block_height - 1;
        let block_hash: BlockHeaderHash = self.get_block_hash(latest_block_height)?;

        let mut database_transaction = DatabaseTransaction::new();

        database_transaction.push(Op::Insert {
            col: COL_META,
            key: KEY_BEST_BLOCK_NUMBER.as_bytes().to_vec(),
            value: update_best_block_num.to_le_bytes().to_vec(),
        });

        database_transaction.push(Op::Delete {
            col: COL_DIGEST,
            key: self.current_digest()?,
        });

        database_transaction.extend(self.remove_block(block_hash)?);

        self.storage.write(database_transaction)?;

        let mut latest_block_height = self.latest_block_height.write();
        *latest_block_height -= 1;

        let mut merkle_tree = self.cm_merkle_tree.write();
        *merkle_tree = self.build_merkle_tree(vec![])?;

        let update_current_digest = DatabaseTransaction(vec![Op::Insert {
            col: COL_META,
            key: KEY_CURR_DIGEST.as_bytes().to_vec(),
            value: to_bytes![merkle_tree.root()]?.to_vec(),
        }]);

        self.storage.write(update_current_digest)?;

        Ok(())
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
