// Copyright (C) 2019-2021 Aleo Systems Inc.
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

use crate::{
    key_value::{KeyValueColumn, Value},
    KeyValueStorage,
};
use snarkvm_dpc::errors::StorageError;

use anyhow::*;
use rocksdb::{ColumnFamily, ColumnFamilyDescriptor, IteratorMode, Options, WriteBatch, DB};
use std::{borrow::Cow, path::Path};

fn convert_err(err: rocksdb::Error) -> StorageError {
    StorageError::Crate("rocksdb", err.to_string())
}

enum Operation {
    Store(KeyValueColumn, Vec<u8>, Vec<u8>),
    Delete(KeyValueColumn, Vec<u8>),
}

pub struct RocksDb {
    db: Option<DB>, // the option is only for Drop (destroy) purposes
    cf_names: Vec<String>,
    current_transaction: Option<Vec<Operation>>,
}

impl KeyValueStorage for RocksDb {
    fn get<'a>(&'a mut self, column: KeyValueColumn, key: &[u8]) -> Result<Option<Value<'a>>> {
        let out = self.db().get_cf(self.get_cf_ref(column as u32), key)?;
        Ok(out.map(Cow::Owned))
    }

    fn exists(&mut self, column: KeyValueColumn, key: &[u8]) -> Result<bool> {
        let out = self.db().get_cf(self.get_cf_ref(column as u32), key)?;
        Ok(out.is_some())
    }

    fn get_column<'a>(&'a mut self, column: KeyValueColumn) -> Result<Vec<(Value<'a>, Value<'a>)>> {
        Ok(self
            .db()
            .iterator_cf(self.get_cf_ref(column as u32), IteratorMode::Start)
            .map(|(key, value)| (Cow::Owned(key.into_vec()), Cow::Owned(value.into_vec())))
            .collect())
    }

    fn get_column_keys<'a>(&'a mut self, column: KeyValueColumn) -> Result<Vec<Value<'a>>> {
        Ok(self
            .db()
            .iterator_cf(self.get_cf_ref(column as u32), IteratorMode::Start)
            .map(|(key, _)| Cow::Owned(key.into_vec()))
            .collect())
    }

    fn store(&mut self, column: KeyValueColumn, key: &[u8], value: &[u8]) -> Result<()> {
        if let Some(transaction) = self.current_transaction.as_mut() {
            transaction.push(Operation::Store(column, key.to_vec(), value.to_vec()));
            return Ok(());
        }
        self.db().put_cf(self.get_cf_ref(column as u32), key, value)?;
        Ok(())
    }

    fn delete(&mut self, column: KeyValueColumn, key: &[u8]) -> Result<()> {
        if let Some(transaction) = self.current_transaction.as_mut() {
            transaction.push(Operation::Delete(column, key.to_vec()));
            return Ok(());
        }
        self.db().delete_cf(self.get_cf_ref(column as u32), key)?;
        Ok(())
    }

    fn begin(&mut self) -> Result<()> {
        if self.current_transaction.is_some() {
            return Err(anyhow!("cannot begin transaction when already in a transaction"));
        }
        self.current_transaction = Some(vec![]);
        Ok(())
    }

    fn abort(&mut self) -> Result<()> {
        if self.current_transaction.is_none() {
            return Err(anyhow!("attempted to abort transaction when none is entered"));
        }
        self.current_transaction = None;
        Ok(())
    }

    fn commit(&mut self) -> Result<()> {
        if self.current_transaction.is_none() {
            return Err(anyhow!("attempted to commit transaction when none is entered"));
        }
        let operations = self.current_transaction.take().unwrap();
        let mut batch = WriteBatch::default();
        for operation in operations {
            match operation {
                Operation::Store(column, key, value) => {
                    batch.put_cf(self.get_cf_ref(column as u32), key, value);
                }
                Operation::Delete(column, key) => {
                    batch.delete_cf(self.get_cf_ref(column as u32), key);
                }
            }
        }
        self.db().write(batch)?;
        Ok(())
    }
}

impl Drop for RocksDb {
    fn drop(&mut self) {
        // as of rocksdb = 0.15, DB::drop must be called before DB::destroy
        let db = self.db.take().unwrap();
        let _path = db.path().to_path_buf();
        drop(db);

        // destroy the database in test conditions
        #[cfg(feature = "test")]
        {
            let _ = DB::destroy(&Options::default(), _path);
        }
    }
}

impl RocksDb {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        RocksDb::open_cf(path, KeyValueColumn::End as u32)
    }

    fn db(&self) -> &DB {
        // safe; always available, only Drop removes it
        self.db.as_ref().unwrap()
    }

    /// Opens storage from the given path with its given names. If storage does not exists,
    /// it creates a new storage file at the given path with its given names, and opens it.
    /// If RocksDB fails to open, returns [StorageError](snarkvm_errors::storage::StorageError).
    fn open_cf<P: AsRef<Path>>(path: P, num_cfs: u32) -> Result<Self, StorageError> {
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

        let storage = DB::open_cf_descriptors(&storage_opts, path, cfs).map_err(convert_err)?;

        Ok(Self {
            db: Some(storage),
            cf_names,
            current_transaction: None,
        })
    }

    /// Returns the column family reference from a given index.
    /// If the given index does not exist, returns [None](std::option::Option).
    fn get_cf_ref(&self, index: u32) -> &ColumnFamily {
        self.db()
            .cf_handle(&self.cf_names[index as usize])
            .expect("the column family exists")
    }
}
