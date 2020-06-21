use crate::{Ledger, TransactionLocation, COL_TRANSACTION_LOCATION};
use snarkos_errors::storage::StorageError;
use snarkos_models::{
    algorithms::MerkleParameters,
    objects::{LedgerScheme, Transaction},
};
use snarkos_objects::BlockHeaderHash;
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    has_duplicates,
    to_bytes,
};

impl<T: Transaction, P: MerkleParameters> Ledger<T, P> {
    /// Returns a transaction location given the transaction ID if it exists. Returns `None` otherwise.
    pub fn get_transaction_location(
        &self,
        transaction_id: &Vec<u8>,
    ) -> Result<Option<TransactionLocation>, StorageError> {
        match self.storage.get(COL_TRANSACTION_LOCATION, &transaction_id)? {
            Some(transaction_locator) => {
                let transaction_location = TransactionLocation::read(&transaction_locator[..])?;
                Ok(Some(transaction_location))
            }
            None => Ok(None),
        }
    }

    /// Returns a transaction given the transaction ID if it exists. Returns `None` otherwise.
    pub fn get_transaction(&self, transaction_id: &Vec<u8>) -> Result<Option<T>, StorageError> {
        match self.get_transaction_location(&transaction_id)? {
            Some(transaction_location) => {
                let block_transactions =
                    self.get_block_transactions(&BlockHeaderHash(transaction_location.block_hash))?;
                Ok(Some(block_transactions.0[transaction_location.index as usize].clone()))
            }
            None => Ok(None),
        }
    }

    /// Returns a transaction in bytes given a transaction ID.
    pub fn get_transaction_bytes(&self, transaction_id: &Vec<u8>) -> Result<Vec<u8>, StorageError> {
        match self.get_transaction(&transaction_id.clone())? {
            Some(transaction) => Ok(to_bytes![transaction]?),
            None => Err(StorageError::InvalidTransactionId(hex::encode(&transaction_id))),
        }
    }

    /// Returns true if the transaction has internal parameters that already exist in the ledger.
    pub fn transcation_conflicts(&self, transaction: &T) -> bool {
        let transaction_serial_numbers = transaction.old_serial_numbers();
        let transaction_commitments = transaction.new_commitments();
        let transaction_memo = transaction.memorandum();

        // Check if the transactions in the block have duplicate serial numbers
        if has_duplicates(transaction_serial_numbers) {
            return true;
        }

        // Check if the transactions in the block have duplicate commitments
        if has_duplicates(transaction_commitments) {
            return true;
        }

        // Check if the transaction memo previously existed in the ledger
        if self.contains_memo(transaction_memo) {
            return true;
        }

        // Check if each transaction serial number previously existed in the ledger
        for sn in transaction_serial_numbers {
            if self.contains_sn(sn) {
                return true;
            }
        }

        // Check if each transaction commitment previously existed in the ledger
        for cm in transaction_commitments {
            if self.contains_cm(cm) {
                return true;
            }
        }

        false
    }
}
