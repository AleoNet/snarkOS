use crate::BlockStorage;
use snarkos_errors::{objects::transaction::TransactionError, storage::StorageError};
use snarkos_objects::{create_script_pub_key, BlockHeaderHash, Outpoint, Transaction, Transactions};
//use snarkos_utilities::{unwrap_option_or_continue, unwrap_result_or_continue};

impl BlockStorage {
    /// Get a transaction bytes given the transaction id.
    pub fn get_transaction_bytes(&self, transaction_id: &Vec<u8>) -> Result<Transaction, StorageError> {
        match self.get_transaction(&transaction_id.clone()) {
            Some(transaction) => Ok(Transaction::deserialize(&transaction)?),
            None => Err(StorageError::InvalidTransactionId(hex::encode(&transaction_id))),
        }
    }

    pub fn is_spent(&self, outpoint: &Outpoint) -> Result<bool, StorageError> {
        unimplemented!()
    }

    /// Ensure that all inputs in all transactions are unspent.
    pub fn check_for_double_spends(&self, transactions: &Transactions) -> Result<(), StorageError> {
        unimplemented!()
    }

    /// Calculate the miner transaction fees from transactions.
    pub fn calculate_transaction_fees(&self, transactions: &Transactions) -> Result<u64, StorageError> {
        unimplemented!()
    }
}
