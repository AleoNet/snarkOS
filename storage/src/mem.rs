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

use crate::NUM_COLS;
use snarkvm_ledger::{DatabaseTransaction, Op, Storage, StorageError};

use parking_lot::RwLock;
use std::{collections::HashMap, path::Path}; // only used for testing

#[allow(clippy::type_complexity)]
pub struct MemDb {
    pub cols: RwLock<Vec<HashMap<Box<[u8]>, Box<[u8]>>>>,
}

impl Storage for MemDb {
    const IN_MEMORY: bool = true;

    fn open(_path: Option<&Path>, _secondary_path: Option<&Path>) -> Result<Self, StorageError> {
        // the paths are just ignored

        Ok(Self {
            cols: RwLock::new(vec![Default::default(); NUM_COLS as usize]),
        })
    }

    fn get(&self, col: u32, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self.cols.read()[col as usize].get(key).map(|v| v.to_vec()))
    }

    #[allow(clippy::type_complexity)]
    fn get_col(&self, col: u32) -> Result<Vec<(Box<[u8]>, Box<[u8]>)>, StorageError> {
        Ok(self.cols.read()[col as usize].clone().into_iter().collect())
    }

    fn get_keys(&self, col: u32) -> Result<Vec<Box<[u8]>>, StorageError> {
        Ok(self.cols.read()[col as usize].keys().cloned().collect())
    }

    fn put<K: AsRef<[u8]>, V: AsRef<[u8]>>(&self, col: u32, key: K, value: V) -> Result<(), StorageError> {
        self.cols.write()[col as usize].insert(key.as_ref().into(), value.as_ref().into());
        Ok(())
    }

    fn batch(&self, transaction: DatabaseTransaction) -> Result<(), StorageError> {
        if transaction.0.is_empty() {
            return Ok(());
        }

        let mut cols = self.cols.write();
        for operation in transaction.0 {
            match operation {
                Op::Insert { col, key, value } => {
                    cols[col as usize].insert(key.into(), value.into());
                }
                Op::Delete { col, key } => {
                    cols[col as usize].remove(&Box::from(key));
                }
            }
        }

        Ok(())
    }

    fn exists(&self, col: u32, key: &[u8]) -> bool {
        self.cols.read()[col as usize].contains_key(key)
    }

    fn try_catch_up_with_primary(&self) -> Result<(), StorageError> {
        // used only in Ledger::catch_up_secondary, doesn't cause an early return
        Err(StorageError::Message("MemDb has no secondary instance".into()))
    }
}
