use crate::*;

use snarkos_algorithms::merkle_tree::MerkleParameters;
use snarkos_errors::storage::StorageError;
use snarkos_objects::{
    dpc::{Block, Transaction},
    BlockHeaderHash,
};
use snarkos_utilities::{bytes::ToBytes, to_bytes};

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

impl<T: Transaction, P: MerkleParameters> BlockStorage<T, P> {
    pub(crate) fn process_transaction(
        &self,
        sn_index: &mut usize,
        cm_index: &mut usize,
        memo_index: &mut usize,
        transaction: &T,
    ) -> Result<(Vec<Op>, Vec<(T::Commitment, usize)>), StorageError> {
        let mut ops = vec![];
        let mut cms = vec![];
        for sn in transaction.old_serial_numbers() {
            if sn != &self.genesis_sn()? {
                let sn_bytes = to_bytes![sn]?;
                if self.get_sn_index(&sn_bytes)?.is_some() {
                    return Err(StorageError::Message("Duplicate sn".into()));
                }

                ops.push(Op::Insert {
                    col: COL_SERIAL_NUMBER,
                    key: sn_bytes,
                    value: (sn_index.clone() as u32).to_le_bytes().to_vec(),
                });
                *sn_index += 1;
            }
        }

        for cm in transaction.new_commitments() {
            let cm_bytes = to_bytes![cm]?;
            if cm == &self.genesis_cm()? || self.get_cm_index(&cm_bytes)?.is_some() {
                return Err(StorageError::Message("Invalid cm".into()));
            }

            ops.push(Op::Insert {
                col: COL_COMMITMENT,
                key: cm_bytes,
                value: (cm_index.clone() as u32).to_le_bytes().to_vec(),
            });
            cms.push((cm.clone(), cm_index.clone()));

            *cm_index += 1;
        }

        if transaction.memorandum() != &self.genesis_memo()? {
            let memo_bytes = to_bytes![transaction.memorandum()]?;
            if self.get_memo_index(&memo_bytes)?.is_some() {
                return Err(StorageError::Message("Duplicate memo".into()));
            } else {
                ops.push(Op::Insert {
                    col: COL_MEMO,
                    key: memo_bytes,
                    value: (memo_index.clone() as u32).to_le_bytes().to_vec(),
                });
                *memo_index += 1;
            }
        }

        Ok((ops, cms))
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

        let mut sn_index = self.current_sn_index()?;
        let mut cm_index = self.current_cm_index()?;
        let mut memo_index = self.current_memo_index()?;

        // Process the individual transactions

        let mut transaction_cms = vec![];

        for (index, transaction) in block.transactions.0.iter().enumerate() {
            let (tx_ops, cms) = self.process_transaction(&mut sn_index, &mut cm_index, &mut memo_index, transaction)?;
            database_transaction.push_vec(tx_ops);
            transaction_cms.extend(cms);

            let transaction_location = TransactionLocation {
                index: index as u32,
                block_hash: block.header.get_hash().0,
            };
            database_transaction.push(Op::Insert {
                col: COL_TRANSACTION_LOCATION,
                key: transaction.transaction_id()?.to_vec(),
                value: to_bytes![transaction_location]?.to_vec(),
            });
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

        // Update the best block number

        let is_genesis = block.header.previous_block_hash == BlockHeaderHash([0u8; 32])
            && self.get_latest_block_height() == 0
            && self.is_empty();

        let mut height = self.latest_block_height.write();
        let mut new_best_block_number = 0;
        if !is_genesis {
            new_best_block_number = *height + 1;
        }

        database_transaction.push(Op::Insert {
            col: COL_META,
            key: KEY_BEST_BLOCK_NUMBER.as_bytes().to_vec(),
            value: new_best_block_number.to_le_bytes().to_vec(),
        });
        database_transaction.push(Op::Insert {
            col: COL_BLOCK_HEADER,
            key: block.header.get_hash().0.to_vec(),
            value: to_bytes![block.header]?.to_vec(),
        });
        database_transaction.push(Op::Insert {
            col: COL_BLOCK_TRANSACTIONS,
            key: block.header.get_hash().0.to_vec(),
            value: to_bytes![block.transactions]?.to_vec(),
        });

        database_transaction.push(Op::Insert {
            col: COL_BLOCK_LOCATOR,
            key: block.header.get_hash().0.to_vec(),
            value: new_best_block_number.to_le_bytes().to_vec(),
        });
        database_transaction.push(Op::Insert {
            col: COL_BLOCK_LOCATOR,
            key: new_best_block_number.to_le_bytes().to_vec(),
            value: block.header.get_hash().0.to_vec(),
        });

        // Dont need to rebuild the tree here because we use an in-memory ledger state for the tree

        // create the new digest and insert into new_digest and past_digests

        // TODO:
        //      CURRENT_DIGEST

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
