use crate::BlockStorage;
use snarkos_errors::storage::StorageError;
use snarkos_objects::dpc::Transaction;

//use std::collections::HashMap;

impl<T: Transaction> BlockStorage<T> {
    /// Get the current commitment index
    pub fn current_cm_index(&self) -> Result<usize, StorageError> {
        unimplemented!()
    }

    /// Get the current serial number index
    pub fn current_sn_index(&self) -> Result<usize, StorageError> {
        unimplemented!()
    }

    /// Get the current memo index
    pub fn current_memo_index(&self) -> Result<usize, StorageError> {
        unimplemented!()
    }

    /// Get the current ledger digest
    pub fn current_digest(&self) -> Result<Vec<u8>, StorageError> {
        unimplemented!()
    }
}
