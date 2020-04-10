use snarkos_errors::storage::StorageError;

use rocksdb::{ColumnFamily, ColumnFamilyDescriptor, Options, DB};
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
    /// Opens storage from the given path. If storage does not exists,
    /// it creates a new storage file at the given path and opens it.
    /// If RocksDB fails to open, returns [StorageError](snarkos_errors::storage::StorageError).
    #[allow(dead_code)]
    pub(crate) fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        Ok(Self {
            storage: Arc::new(DB::open_default(path)?),
            cf_names: vec![],
        })
    }

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

            cfs.push(ColumnFamilyDescriptor::new(column_name.clone(), cf_opts));
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
    pub(crate) fn get_cf_ref(&self, index: usize) -> Option<&ColumnFamily> {
        self.storage.cf_handle(&self.cf_names[index])
    }

    /// Returns the value from a given key.
    /// If the given key does not exist, returns [StorageError](snarkos_errors::storage::StorageError).
    pub(crate) fn get(&self, key: &Vec<u8>) -> Result<Option<Vec<u8>>, StorageError> {
        //        let col_key = ColKey::from(key);

        //        let val = match self.get_cf_ref(col_key.column as usize) {
        //            Some(cf) => self.storage.get_cf(cf, col_key.key)?,
        //            None => return Err(StorageError::InvalidColumnFamily(col_key.column)),
        //        };
        //
        //        Ok(val)
        unimplemented!()
    }

    /// Returns `Ok(())` after storing a given [KeyValue](snarkos_storage::key_value::KeyValue).
    /// If the column does not exist, returns [StorageError](snarkos_errors::storage::StorageError).
    pub(crate) fn insert(&self, key: Vec<u8>, value: Vec<u8>) -> Result<(), StorageError> {
        //        let col_key_value = ColKeyValue::from(&key_value);
        //
        //        match self.get_cf_ref(col_key_value.column as usize) {
        //            Some(cf) => self.storage.put_cf(cf, col_key_value.key, col_key_value.value)?,
        //            None => return Err(StorageError::InvalidColumnFamily(col_key_value.column)),
        //        }
        //
        //        Ok(())
        unimplemented!()
    }

    /// Returns `Ok(())` after storing a given list of [KeyValue](snarkos_storage::key_value::KeyValue).
    /// If any column does not exist, returns [StorageError](snarkos_errors::storage::StorageError).
    pub(crate) fn insert_batch(&self, _key: &Vec<u8>, _values: &Vec<u8>) -> Result<(), StorageError> {
        unimplemented!()
    }

    /// Returns `Ok(())` after removing a given [Key](snarkos_storage::key_value::Key).
    /// If the column does not exist, returns [StorageError](snarkos_errors::storage::StorageError).
    pub(crate) fn remove(&self, _key: &Vec<u8>) -> Result<(), StorageError> {
        //        let col_key = ColKey::from(key);
        //
        //        match self.get_cf_ref(col_key.column as usize) {
        //            Some(cf) => self.storage.delete_cf(cf, col_key.key)?,
        //            None => return Err(StorageError::InvalidColumnFamily(col_key.column)),
        //        };
        //
        //        Ok(())

        unimplemented!()
    }

    /// Returns `Ok(())` after removing a given list of [Key](snarkos_storage::key_value::Key).
    /// If any column does not exist, returns [StorageError](snarkos_errors::storage::StorageError).
    pub fn remove_batch(&self, keys: Vec<Vec<u8>>) -> Result<(), StorageError> {
        unimplemented!()
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
