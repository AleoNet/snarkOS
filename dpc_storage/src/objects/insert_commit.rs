use crate::{BlockStorage, DatabaseTransaction, Op, SideChainPath};
use snarkos_errors::storage::StorageError;
use snarkos_objects::{
    dpc::{Block, Transaction},
    BlockHeaderHash,
};

use std::{collections::HashSet, hash::Hash};

/// Check if an iterator has duplicate elements
pub fn has_duplicates<T>(iter: T) -> bool
where
    T: IntoIterator,
    T::Item: Eq + Hash,
{
    let mut uniq = HashSet::new();
    !iter.into_iter().all(move |x| uniq.insert(x))
}

impl<T: Transaction> BlockStorage<T> {
    pub fn process_transaction(&self, transaction: &T) -> Result<Vec<Op>, StorageError> {
        //        let mut cur_sn_index =;

        Ok(vec![])
    }

    pub fn insert_block(&self, block: Block<T>) -> Result<(), StorageError> {
        let latest_block_height = self.get_latest_block_height();

        let mut database_transaction = DatabaseTransaction::new();

        let mut transaction_serial_numbers = vec![];
        let mut transaction_commitments = vec![];
        let mut transaction_memos = vec![];

        for transaction in &block.transactions.0 {
            transaction_serial_numbers.push(transaction.transaction_id()?);
            transaction_commitments.push(transaction.new_commitments());
            transaction_memos.push(transaction.memorandum());
        }

        // Sanitize the block inputs

        // Check if the transactions in the block have duplicate serial numbers
        if has_duplicates(transaction_serial_numbers) {
            return Err(StorageError::Message("Duplicate serial numbers".into()));
        }

        // Check if the transactions in the block have duplicate commitments
        if has_duplicates(transaction_commitments) {
            return Err(StorageError::Message("Duplicate commitments".into()));
        }

        // Check if the transactions in the block have duplicate memos
        if has_duplicates(transaction_memos) {
            return Err(StorageError::Message("Duplicate transaction memos".into()));
        }

        // Process the individual transactions
        for transaction in &block.transactions.0 {
            let tx_ops = self.process_transaction(transaction)?;
            database_transaction.push_vec(tx_ops);
        }

        // Dont need to rebuild the tree here because we use an in-memory ledger state for the tree

        // TODO:
        //      BEST_BLOCK_NUMBER, CURRENT_CM_INDEX, CURRENT_SN_INDEX, CURRENT_MEMO_INDEX, CURRENT_DIGEST

        Ok(())
    }

    /// Commit/canonize a particular block.
    pub fn commit(&self, _block_header_hash: BlockHeaderHash) -> Result<(), StorageError> {
        unimplemented!()
    }

    /// Insert a block into the storage and commit as part of the longest chain.
    pub fn insert_and_commit(&self, _block: Block<T>) -> Result<(), StorageError> {
        unimplemented!()
    }

    /// Returns true if the block exists in the canon chain.
    pub fn is_canon(&self, _block_hash: &BlockHeaderHash) -> bool {
        unimplemented!()
    }

    /// Returns true if the block corresponding to this block's previous_block_h.is_canon(&block_haash is in the canon chain.
    pub fn is_previous_block_canon(&self, _block: Block<T>) -> bool {
        unimplemented!()
    }

    /// Revert the chain to the state before the fork.
    pub fn revert_for_fork(&self, _side_chain_path: &SideChainPath) -> Result<(), StorageError> {
        unimplemented!()
    }
}
