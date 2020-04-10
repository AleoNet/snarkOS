use crate::BlockStorage;
use snarkos_errors::storage::StorageError;
use snarkos_objects::{Transaction, Transactions};
//use snarkos_utilities::{unwrap_option_or_continue, unwrap_result_or_continue};

impl BlockStorage {
    /// Get a transaction bytes given the transaction id.
    pub fn get_transaction_bytes(&self, transaction_id: &Vec<u8>) -> Result<Transaction, StorageError> {
        match self.get_transaction(&transaction_id.clone()) {
            Some(transaction) => Ok(Transaction::deserialize(&transaction)?),
            None => Err(StorageError::InvalidTransactionId(hex::encode(&transaction_id))),
        }
    }

    /// Calculate the miner transaction fees from transactions.
    pub fn calculate_transaction_fees(&self, _transactions: &Transactions) -> Result<u64, StorageError> {
        unimplemented!()
    }
}
