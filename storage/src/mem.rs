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

use std::borrow::Cow;

use crate::{
    key_value::{KeyValueColumn, Value},
    KeyValueStorage,
};
use anyhow::*;
use indexmap::IndexMap;

type Store = IndexMap<KeyValueColumn, IndexMap<Vec<u8>, Vec<u8>>>;

/// used in tests as a memory-only DB
#[derive(Default)]
pub struct MemDb {
    // incredibly naive transaction model: copy the database proactively and reset if the transaction is aborted
    transaction: Option<Store>,
    entries: Store,
}

impl MemDb {
    pub fn new() -> Self {
        MemDb {
            transaction: None,
            entries: IndexMap::new(),
        }
    }

    fn column(&mut self, column: KeyValueColumn) -> &mut IndexMap<Vec<u8>, Vec<u8>> {
        self.entries.entry(column).or_insert_with(IndexMap::new)
    }

    fn transaction_column(&mut self, column: KeyValueColumn) -> Option<&mut IndexMap<Vec<u8>, Vec<u8>>> {
        if let Some(transaction) = self.transaction.as_mut() {
            Some(transaction.entry(column).or_insert_with(IndexMap::new))
        } else {
            None
        }
    }
}

impl KeyValueStorage for MemDb {
    fn get<'a>(&'a mut self, column: KeyValueColumn, key: &[u8]) -> Result<Option<Value<'a>>> {
        match self.column(column).get(key) {
            Some(value) => Ok(Some(Cow::Borrowed(&value[..]))),
            None => Ok(None),
        }
    }

    fn exists(&mut self, column: KeyValueColumn, key: &[u8]) -> Result<bool> {
        Ok(self.column(column).contains_key(key))
    }

    fn get_column_keys<'a>(&'a mut self, column: KeyValueColumn) -> Result<Vec<Value<'a>>> {
        Ok(self.column(column).keys().map(|x| Cow::Borrowed(&x[..])).collect())
    }

    fn get_column<'a>(&'a mut self, column: KeyValueColumn) -> Result<Vec<(Value<'a>, Value<'a>)>> {
        Ok(self
            .column(column)
            .iter()
            .map(|(key, value)| (Cow::Borrowed(&key[..]), Cow::Borrowed(&value[..])))
            .collect())
    }

    fn store(&mut self, column: KeyValueColumn, key: &[u8], value: &[u8]) -> Result<()> {
        if let Some(column) = self.transaction_column(column) {
            column.insert(key.to_vec(), value.to_vec());
            return Ok(());
        }
        self.column(column).insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn delete(&mut self, column: KeyValueColumn, key: &[u8]) -> Result<()> {
        if let Some(column) = self.transaction_column(column) {
            column.remove(key);
            return Ok(());
        }
        self.column(column).remove(key);
        Ok(())
    }

    fn in_transaction(&self) -> bool {
        self.transaction.is_some()
    }

    fn begin(&mut self) -> Result<()> {
        if self.in_transaction() {
            return Err(anyhow!("attempted to restart a transaction"));
        }
        self.transaction = Some(self.entries.clone());
        Ok(())
    }

    fn abort(&mut self) -> Result<()> {
        if !self.in_transaction() {
            return Err(anyhow!("attempted to abort when not in a transaction"));
        }
        self.transaction = None;
        Ok(())
    }

    fn commit(&mut self) -> Result<()> {
        if !self.in_transaction() {
            return Err(anyhow!("attempted to commit when not in a transaction"));
        }
        self.entries = self.transaction.take().unwrap();
        Ok(())
    }
}
