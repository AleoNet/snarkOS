use crate::*;
use snarkos_errors::storage::StorageError;
use snarkos_models::{algorithms::LoadableMerkleParameters, dpc::Record, objects::Transaction};
use snarkos_utilities::{
    bytes::{FromBytes, ToBytes},
    to_bytes,
};

// TODO (howardwu): Remove this from `Ledger` as it is not used for ledger state.
//  This is merely for local node / miner functionality.
impl<T: Transaction, P: LoadableMerkleParameters> Ledger<T, P> {
    /// Get all stored record commitments of the node
    pub fn get_record_commitments(&self, limit: usize) -> Result<Vec<Vec<u8>>, StorageError> {
        let mut record_commitments = vec![];

        for (commitment_key, _record) in self.storage.get_iter(COL_RECORDS)? {
            if record_commitments.len() >= limit {
                break;
            }

            record_commitments.push(commitment_key.to_vec());
        }

        Ok(record_commitments)
    }

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
        // TODO (raychu86) No need to store dummy records
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

    /// Removes a record from storage.
    pub fn delete_record<R: Record>(&self, record: R) -> Result<(), StorageError> {
        let mut database_transaction = DatabaseTransaction::new();

        database_transaction.push(Op::Delete {
            col: COL_RECORDS,
            key: to_bytes![record.commitment()]?.to_vec(),
        });

        self.storage.write(database_transaction)
    }
}
