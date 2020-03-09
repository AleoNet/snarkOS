use crate::{BlockStorage, KeyValue, SideChainPath, TransactionMeta, TransactionValue, KEY_BEST_BLOCK_NUMBER};
use snarkos_errors::{
    objects::{BlockError, TransactionError},
    storage::StorageError,
};
use snarkos_objects::{Block, BlockHeaderHash};

use std::collections::HashMap;

/// Protocol methods for inserting blocks to storage and committing them to the blockchain.
/// Blocks that extend the canonical (longest) chain are inserted and committed in the database.
/// All other sidechain blocks are inserted into the database but not committed.
/// This protocol allows for easy forking if a sidechain becomes longer than the canonical chain.
/// Currently there is no feature to routinely remove old sidechain blocks.
impl BlockStorage {
    /// Insert a block into the storage but do not commit.
    pub fn insert_only(&self, block: Block) -> Result<(), StorageError> {
        // Verify that the block does not already exist in storage.
        if self.block_hash_exists(&block.header.get_hash()) {
            return Err(StorageError::BlockError(BlockError::BlockExists(
                block.header.get_hash().to_string(),
            )));
        }

        let transaction_ids: Vec<Vec<u8>> = block.transactions.to_transaction_ids().unwrap();
        let transaction_bytes: Vec<Vec<u8>> = block.transactions.serialize().unwrap();

        let mut transactions_to_store = vec![];
        for (index, tx_bytes) in transaction_bytes.iter().enumerate() {
            let transaction_value = match self.get_transaction(&transaction_ids[index]) {
                Some(transaction_value) => transaction_value.increment(),
                None => TransactionValue::new(tx_bytes.clone()),
            };

            transactions_to_store.push(KeyValue::Transactions(
                transaction_ids[index].clone(),
                transaction_value,
            ));

            transactions_to_store.push(KeyValue::TransactionMeta(
                transaction_ids[index].clone(),
                TransactionMeta {
                    spent: vec![false; block.transactions[index].parameters.outputs.len()],
                },
            ));
        }

        let block_header_hash = block.header.get_hash();
        let block_transactions = KeyValue::BlockTransactions(block_header_hash.clone(), transaction_ids);
        let child_hashes = KeyValue::ChildHashes(block.header.previous_block_hash.clone(), block_header_hash.clone());
        let block_header = KeyValue::BlockHeaders(block_header_hash, block.header);

        self.storage
            .insert_batch(vec![block_header, block_transactions, child_hashes])?;
        self.storage.insert_batch(transactions_to_store)?;

        Ok(())
    }

    /// Commit/canonize a particular block.
    pub fn commit(&self, block_header_hash: BlockHeaderHash) -> Result<(), StorageError> {
        let block = self.get_block(&block_header_hash.clone())?;

        let is_genesis = block.header.previous_block_hash == BlockHeaderHash([0u8; 32])
            && self.get_latest_block_height() == 0
            && self.is_empty();

        if !is_genesis {
            let latest_block = self.get_latest_block()?;

            if latest_block.header.get_hash() != block.header.previous_block_hash {
                return Err(StorageError::BlockError(BlockError::InvalidParent(
                    latest_block.header.get_hash().to_string(),
                    block.header.previous_block_hash.to_string(),
                )));
            }
        }

        // Update transaction spent status

        let mut transaction_meta_updates: HashMap<Vec<u8>, TransactionMeta> = HashMap::new();
        for transaction in block.transactions.iter() {
            for input in &transaction.parameters.inputs {
                if input.outpoint.is_coinbase() {
                    continue;
                }

                let mut new_transaction_meta = match transaction_meta_updates.get(&input.outpoint.transaction_id) {
                    Some(transaction_meta) => transaction_meta.clone(),
                    None => self.get_transaction_meta(&input.outpoint.transaction_id)?,
                };

                if new_transaction_meta.spent[input.outpoint.index as usize] {
                    return Err(StorageError::TransactionError(TransactionError::DoubleSpend(
                        hex::encode(&input.outpoint.transaction_id),
                    )));
                }

                new_transaction_meta.spent[input.outpoint.index as usize] = true;
                transaction_meta_updates.insert(input.outpoint.transaction_id.clone(), new_transaction_meta);
            }
        }

        let mut update_spent_transactions = vec![];
        for (txid, transaction_meta) in transaction_meta_updates {
            update_spent_transactions.push(KeyValue::TransactionMeta(txid, transaction_meta));
        }

        // Handle storage inserts and height update

        let mut height = self.latest_block_height.write();
        let mut new_best_block_number = 0;
        if !is_genesis {
            new_best_block_number = *height + 1;
        }

        let best_block_number = KeyValue::Meta(KEY_BEST_BLOCK_NUMBER, new_best_block_number.to_le_bytes().to_vec());
        let block_hash = KeyValue::BlockHashes(new_best_block_number, block_header_hash.clone());
        let block_numbers = KeyValue::BlockNumbers(block_header_hash, new_best_block_number);

        self.storage
            .insert_batch(vec![best_block_number, block_hash, block_numbers])?;
        self.storage.insert_batch(update_spent_transactions)?;

        if !is_genesis {
            *height += 1;
        }

        Ok(())
    }

    /// Insert a block into the storage and commit as part of the longest chain.
    pub fn insert_and_commit(&self, block: Block) -> Result<(), StorageError> {
        let block_hash = block.header.get_hash();

        // If the block does not exist in the storage
        if !self.block_hash_exists(&block_hash) {
            // Insert it first
            self.insert_only(block)?;
        }
        // Commit it
        self.commit(block_hash)
    }

    /// Returns true if the block exists in the canon chain.
    pub fn is_canon(&self, block_hash: &BlockHeaderHash) -> bool {
        self.block_hash_exists(block_hash) && self.get_block_num(block_hash).is_ok()
    }

    /// Returns true if the block corresponding to this block's previous_block_hash is in the canon chain.
    pub fn is_previous_block_canon(&self, block: &Block) -> bool {
        self.is_canon(&block.header.previous_block_hash)
    }

    /// Revert the chain to the state before the fork.
    pub fn revert_for_fork(&self, side_chain_path: &SideChainPath) -> Result<(), StorageError> {
        let latest_block_height = self.get_latest_block_height();

        if side_chain_path.new_block_number > latest_block_height {
            for _ in (side_chain_path.shared_block_number)..latest_block_height {
                self.remove_latest_block()?;
            }
        }

        Ok(())
    }
}
