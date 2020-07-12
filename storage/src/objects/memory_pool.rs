use crate::{DatabaseTransaction, Ledger, Op, COL_META, KEY_MEMORY_POOL};
use snarkos_errors::storage::StorageError;
use snarkos_models::{algorithms::LoadableMerkleParameters, objects::Transaction};

impl<T: Transaction, P: LoadableMerkleParameters> Ledger<T, P> {
    /// Get the stored memory pool transactions.
    pub fn get_memory_pool(&self) -> Result<Vec<u8>, StorageError> {
        Ok(self.get(COL_META, &KEY_MEMORY_POOL.as_bytes().to_vec())?)
    }

    /// Store the memory pool transactions.
    pub fn store_to_memory_pool(&self, transactions_serialized: Vec<u8>) -> Result<(), StorageError> {
        let op = Op::Insert {
            col: COL_META,
            key: KEY_MEMORY_POOL.as_bytes().to_vec(),
            value: transactions_serialized,
        };
        self.storage.write(DatabaseTransaction(vec![op]))
    }
}
