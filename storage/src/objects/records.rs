use crate::*;
use snarkos_algorithms::merkle_tree::MerkleParameters;
use snarkos_errors::storage::StorageError;
use snarkos_models::{dpc::Record, objects::Transaction};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

impl<T: Transaction, P: MerkleParameters> BlockStorage<T, P> {
    /// Get a transaction bytes given the transaction id.
    pub fn get_record<R: Record>(&self, record_commitment: &Vec<u8>) -> Result<Option<R>, StorageError> {
        match self.storage.get(COL_RECORDS, &record_commitment)? {
            Some(record_bytes) => {
                let record: R = FromBytes::read(&record_bytes[..])?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// Get a transaction bytes given the transaction id.
    pub fn store_record<R: Record>(&self, record: &R) -> Result<(), StorageError> {
        let mut database_transaction = DatabaseTransaction::new();

        database_transaction.push(Op::Insert {
            col: COL_RECORDS,
            key: to_bytes![record.commitment()]?.to_vec(),
            value: to_bytes![record]?.to_vec(),
        });

        self.storage.write(database_transaction)
    }

    /// Get a transaction bytes given the transaction id.
    pub fn store_records<R: Record>(&self, records: &Vec<R>) -> Result<(), StorageError> {
        let mut database_transaction = DatabaseTransaction::new();

        for record in records {
            database_transaction.push(Op::Insert {
                col: COL_RECORDS,
                key: to_bytes![record.commitment()]?.to_vec(),
                value: to_bytes![record]?.to_vec(),
            });
        }

        self.storage.write(database_transaction)
    }

    /// Remove a record from storage
    pub fn delete_record<R: Record>(&self, record: R) -> Result<(), StorageError> {
        let mut database_transaction = DatabaseTransaction::new();

        database_transaction.push(Op::Delete {
            col: COL_RECORDS,
            key: to_bytes![record.commitment()]?.to_vec(),
        });

        self.storage.write(database_transaction)
    }
}
