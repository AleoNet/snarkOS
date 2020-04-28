use crate::{DatabaseTransaction, Op};
use snarkos_errors::storage::StorageError;

use rocksdb::{ColumnFamily, ColumnFamilyDescriptor, DBIterator, IteratorMode, Options, WriteBatch, DB};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// A low-level struct for storing state used by the system.
#[derive(Clone)]
pub struct Storage {
    pub storage: Arc<DB>,
    pub cf_names: Vec<String>,
}

impl Storage {
    //    /// Opens storage from the given path. If storage does not exists,
    //    /// it creates a new storage file at the given path and opens it.
    //    /// If RocksDB fails to open, returns [StorageError](snarkos_errors::storage::StorageError).
    //    #[allow(dead_code)]
    //    pub(crate) fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
    //        Ok(Self {
    //            storage: Arc::new(DB::open_default(path)?),
    //            cf_names: vec![],
    //        })
    //    }

    /// Opens storage from the given path with its given names. If storage does not exists,
    /// it creates a new storage file at the given path with its given names, and opens it.
    /// If RocksDB fails to open, returns [StorageError](snarkos_errors::storage::StorageError).
    pub(crate) fn open_cf<P: AsRef<Path>>(path: P, num_cfs: u32) -> Result<Self, StorageError> {
        let mut cfs = Vec::with_capacity(num_cfs as usize);
        let mut cf_names: Vec<String> = Vec::with_capacity(cfs.len());

        for column in 0..num_cfs {
            let column_name = format!("col{}", column.to_string());

            let mut cf_opts = Options::default();
            cf_opts.set_max_write_buffer_number(16);

            cfs.push(ColumnFamilyDescriptor::new(&column_name, cf_opts));
            cf_names.push(column_name);
        }

        let mut storage_opts = Options::default();
        storage_opts.increase_parallelism(3);
        storage_opts.create_missing_column_families(true);
        storage_opts.create_if_missing(true);

        let storage = Arc::new(DB::open_cf_descriptors(&storage_opts, path, cfs)?);

        Ok(Self { storage, cf_names })
    }

    /// Returns the column family reference from a given index.
    /// If the given index does not exist, returns [None](std::option::Option).
    pub(crate) fn get_cf_ref(&self, index: u32) -> &ColumnFamily {
        self.storage
            .cf_handle(&self.cf_names[index as usize])
            .expect("the column family exists")
    }

    /// Returns the value from a given key and col.
    /// If the given key does not exist, returns [StorageError](snarkos_errors::storage::StorageError).
    pub(crate) fn get(&self, col: u32, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.storage.get_cf(self.get_cf_ref(col), key)?)
    }

    /// Returns the iterator from a given col.
    /// If the given key does not exist, returns [StorageError](snarkos_errors::storage::StorageError).
    pub(crate) fn get_iter(&self, col: u32) -> Result<DBIterator, StorageError> {
        Ok(self.storage.iterator_cf(self.get_cf_ref(col), IteratorMode::Start)?)
    }

    /// Returns `Ok(())` after executing a database transaction
    /// If the any of the operations fail, returns [StorageError](snarkos_errors::storage::StorageError).
    pub(crate) fn write(&self, transaction: DatabaseTransaction) -> Result<(), StorageError> {
        let mut batch = WriteBatch::default();

        for operation in transaction.0 {
            match operation {
                Op::Insert { col, key, value } => {
                    let cf = self.get_cf_ref(col);
                    batch.put_cf(cf, &key, value)?;
                }
                Op::Delete { col, key } => {
                    let cf = self.get_cf_ref(col);
                    batch.delete_cf(cf, &key)?;
                }
            };
        }

        self.storage.write(batch)?;

        Ok(())
    }

    /// Returns true if a value exists for a key and col pair.
    pub fn exists(&self, col: u32, key: &[u8]) -> bool {
        match self.storage.get_cf(self.get_cf_ref(col), key) {
            Ok(val) => val.is_some(),
            Err(_) => false,
        }
    }

    /// Returns `Ok(())` after destroying the storage
    /// If RocksDB fails to destroy storage, returns [StorageError](snarkos_errors::storage::StorageError).
    pub fn destroy(&self) -> Result<(), StorageError> {
        let path = self.storage.path();
        drop(&self.storage);
        Self::destroy_storage(path.into())
    }

    /// Returns `Ok(())` after destroying the storage of the given path.
    /// If RocksDB fails to destroy storage, returns [StorageError](snarkos_errors::storage::StorageError).
    pub(crate) fn destroy_storage(path: PathBuf) -> Result<(), StorageError> {
        let mut storage_opts = Options::default();
        storage_opts.create_missing_column_families(true);
        storage_opts.create_if_missing(true);

        Ok(DB::destroy(&storage_opts, path)?)
    }
}
