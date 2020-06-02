use crate::{has_duplicates, Ledger};
use snarkos_errors::storage::StorageError;
use snarkos_models::{
    algorithms::MerkleParameters,
    objects::{LedgerScheme, Transaction},
};
use snarkos_objects::dpc::DPCTransactions;
use snarkos_utilities::{bytes::ToBytes, to_bytes};

impl<T: Transaction, P: MerkleParameters> Ledger<T, P> {
    /// Get a transaction bytes given the transaction id.
    pub fn get_transaction_bytes(&self, transaction_id: &Vec<u8>) -> Result<Vec<u8>, StorageError> {
        match self.get_transaction(&transaction_id.clone())? {
            Some(transaction) => Ok(to_bytes![transaction]?),
            None => Err(StorageError::InvalidTransactionId(hex::encode(&transaction_id))),
        }
    }

    pub fn transcation_conflicts(&self, transaction: &T) -> Result<bool, StorageError> {
        let transaction_serial_numbers = transaction.old_serial_numbers();
        let transaction_commitments = transaction.new_commitments();
        let transaction_memo = transaction.memorandum();

        // Check if the transactions in the block have duplicate serial numbers
        if has_duplicates(transaction_serial_numbers) {
            return Ok(true);
        }

        // Check if the transactions in the block have duplicate commitments
        if has_duplicates(transaction_commitments) {
            return Ok(true);
        }

        if self.contains_memo(transaction_memo) {
            return Ok(true);
        }

        for sn in transaction_serial_numbers {
            if self.contains_sn(sn) {
                return Ok(true);
            }
        }

        for cm in transaction_commitments {
            if self.contains_cm(cm) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Calculate the miner transaction fees from transactions.
    pub fn calculate_transaction_fees(&self, transactions: &DPCTransactions<T>) -> Result<u64, StorageError> {
        let mut balance = 0;

        for transaction in transactions.iter() {
            let value_balance = transaction.value_balance();

            // Only add to the transaction fee if the transaction is not a coinbase transaction
            if !value_balance.is_negative() {
                balance += value_balance as u64;
            }
        }

        Ok(balance)
    }
}
