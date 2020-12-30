// Copyright (C) 2019-2020 Aleo Systems Inc.
// This file is part of the snarkOS library.

// The snarkOS library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkOS library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkOS library. If not, see <https://www.gnu.org/licenses/>.

use crate::{error::StorageError, DatabaseTransaction, Op};

use rocksdb::{ColumnFamily, ColumnFamilyDescriptor, DBIterator, IteratorMode, Options, WriteBatch, DB};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// A low-level struct for storing state used by the system.
#[derive(Clone)]
pub struct Storage {
    pub db: Arc<DB>,
    pub cf_names: Vec<String>,
}

impl Storage {
    /// Opens storage from the given path with its given names. If storage does not exists,
    /// it creates a new storage file at the given path with its given names, and opens it.
    /// If RocksDB fails to open, returns [StorageError](snarkvm_errors::storage::StorageError).
    pub fn open_cf<P: AsRef<Path>>(path: P, num_cfs: u32) -> Result<Self, StorageError> {
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

        Ok(Self { db: storage, cf_names })
    }

    /// Opens a secondary storage instance from the given path with its given names.
    /// If RocksDB fails to open, returns [StorageError](snarkvm_errors::storage::StorageError).
    pub fn open_secondary_cf<P: AsRef<Path> + Clone>(
        primary_path: P,
        secondary_path: P,
        num_cfs: u32,
    ) -> Result<Self, StorageError> {
        let mut cf_names: Vec<String> = Vec::with_capacity(num_cfs as usize);

        for column in 0..num_cfs {
            let column_name = format!("col{}", column.to_string());

            cf_names.push(column_name);
        }

        let mut storage_opts = Options::default();
        storage_opts.increase_parallelism(2);

        let storage = Arc::new(DB::open_cf_as_secondary(
            &storage_opts,
            primary_path,
            secondary_path,
            cf_names.clone(),
        )?);

        storage.try_catch_up_with_primary()?;

        Ok(Self { db: storage, cf_names })
    }

    /// Returns the column family reference from a given index.
    /// If the given index does not exist, returns [None](std::option::Option).
    pub(crate) fn get_cf_ref(&self, index: u32) -> &ColumnFamily {
        self.db
            .cf_handle(&self.cf_names[index as usize])
            .expect("the column family exists")
    }

    /// Returns the value from a given key and col.
    /// If the given key does not exist, returns [StorageError](snarkvm_errors::storage::StorageError).
    pub(crate) fn get(&self, col: u32, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.db.get_cf(self.get_cf_ref(col), key)?)
    }

    /// Returns the iterator from a given col.
    /// If the given key does not exist, returns [StorageError](snarkvm_errors::storage::StorageError).
    pub(crate) fn get_iter(&self, col: u32) -> Result<DBIterator, StorageError> {
        Ok(self.db.iterator_cf(self.get_cf_ref(col), IteratorMode::Start))
    }

    /// Returns `Ok(())` after executing a database transaction
    /// If the any of the operations fail, returns [StorageError](snarkvm_errors::storage::StorageError).
    pub(crate) fn write(&self, transaction: DatabaseTransaction) -> Result<(), StorageError> {
        let mut batch = WriteBatch::default();

        for operation in transaction.0 {
            match operation {
                Op::Insert { col, key, value } => {
                    let cf = self.get_cf_ref(col);
                    batch.put_cf(cf, &key, value);
                }
                Op::Delete { col, key } => {
                    let cf = self.get_cf_ref(col);
                    batch.delete_cf(cf, &key);
                }
            };
        }

        self.db.write(batch)?;

        Ok(())
    }

    /// Returns true if a value exists for a key and col pair.
    pub fn exists(&self, col: u32, key: &[u8]) -> bool {
        match self.db.get_cf(self.get_cf_ref(col), key) {
            Ok(val) => val.is_some(),
            Err(_) => false,
        }
    }

    /// Returns `Ok(())` after destroying the storage
    /// If RocksDB fails to destroy storage, returns [StorageError](snarkvm_errors::storage::StorageError).
    pub fn destroy(&self) -> Result<(), StorageError> {
        let path = self.db.path();
        // drop(&self.db); FIXME: this didn't actually drop self.db
        Self::destroy_storage(path.into())
    }

    /// Returns `Ok(())` after destroying the storage of the given path.
    /// If RocksDB fails to destroy storage, returns [StorageError](snarkvm_errors::storage::StorageError).
    pub(crate) fn destroy_storage(path: PathBuf) -> Result<(), StorageError> {
        let mut storage_opts = Options::default();
        storage_opts.create_missing_column_families(true);
        storage_opts.create_if_missing(true);

        Ok(DB::destroy(&storage_opts, path)?)
    }
}
