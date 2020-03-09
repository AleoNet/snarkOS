use crate::{BlockStorage, Key, KeyValue, TransactionMeta, KEY_BEST_BLOCK_NUMBER};
use snarkos_errors::storage::StorageError;
use snarkos_objects::{Block, BlockHeader, BlockHeaderHash, Transactions};
use std::collections::HashMap;

impl BlockStorage {
    /// Get a block given the block hash.
    pub fn get_block(&self, block_hash: &BlockHeaderHash) -> Result<Block, StorageError> {
        let block_transactions = self.get_block_transactions(block_hash)?;

        let mut transactions = vec![];
        for block_transaction_id in block_transactions {
            transactions.push(self.get_transaction_bytes(&block_transaction_id).unwrap());
        }

        Ok(Block {
            header: self.get_block_header(&block_hash)?,
            transactions: Transactions::from(&transactions),
        })
    }

    /// Get a block given the block number.
    pub fn get_block_from_block_num(&self, block_num: u32) -> Result<Block, StorageError> {
        if block_num > self.get_latest_block_height() {
            return Err(StorageError::InvalidBlockNumber(block_num));
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
        let block_transactions: Vec<Vec<u8>> = self.get_block_transactions(&block_hash)?;

        for block_transaction_id in block_transactions {
            self.decrement_transaction_value(&block_transaction_id)?;
        }

        self.storage.remove_batch(vec![
            Key::BlockHeaders(block_hash.clone()),
            Key::BlockTransactions(block_hash),
        ])?;

        Ok(())
    }

    /// Remove the latest block.
    pub fn remove_latest_block(&self) -> Result<(), StorageError> {
        // De-commit the block from the valid chain

        let latest_block_height = self.get_latest_block_height();
        if latest_block_height == 0 {
            return Err(StorageError::InvalidBlockRemovalNum(0, 0));
        }

        let block_hash: BlockHeaderHash = self.get_block_hash(latest_block_height)?;
        let block_transactions: Vec<Vec<u8>> = self.get_block_transactions(&block_hash)?;

        let mut transaction_meta_updates: HashMap<Vec<u8>, TransactionMeta> = HashMap::new();

        for block_transaction_id in block_transactions {
            // Update transaction meta spends

            for input in self
                .get_transaction_bytes(&block_transaction_id)
                .unwrap()
                .parameters
                .inputs
            {
                if input.outpoint.is_coinbase() {
                    continue;
                }

                let mut new_transaction_meta = match transaction_meta_updates.get(&input.outpoint.transaction_id) {
                    Some(transaction_meta) => transaction_meta.clone(),
                    None => self.get_transaction_meta(&input.outpoint.transaction_id)?,
                };

                new_transaction_meta.spent[input.outpoint.index as usize] = false;
                transaction_meta_updates.insert(input.outpoint.transaction_id.clone(), new_transaction_meta);
            }
        }

        // Update spent status of relevant utxos

        let mut update_spent_transactions = vec![];
        for (txid, transaction_meta) in transaction_meta_updates {
            update_spent_transactions.push(KeyValue::TransactionMeta(txid, transaction_meta));
        }

        let update_best_block_num = latest_block_height - 1;
        let best_block_number = KeyValue::Meta(KEY_BEST_BLOCK_NUMBER, (update_best_block_num).to_le_bytes().to_vec());

        let mut storage_inserts = vec![best_block_number];
        storage_inserts.extend(update_spent_transactions);

        self.storage.insert_batch(storage_inserts)?;
        self.storage.remove_batch(vec![
            Key::BlockHashes(latest_block_height),
            Key::BlockNumbers(block_hash.clone()),
        ])?;

        let mut latest_block_height = self.latest_block_height.write();
        *latest_block_height -= 1;

        // Remove the block structure

        self.remove_block(block_hash)?;

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
