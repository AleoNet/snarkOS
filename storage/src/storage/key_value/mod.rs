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

use anyhow::*;

mod column;
pub use column::*;

mod store;
pub use store::KeyValueStore;

pub type Value<'a> = Cow<'a, [u8]>;

pub trait KeyValueStorage {
    fn get<'a>(&'a mut self, column: KeyValueColumn, key: &[u8]) -> Result<Option<Value<'a>>>;

    fn exists(&mut self, column: KeyValueColumn, key: &[u8]) -> Result<bool>;

    fn get_column_keys<'a>(&'a mut self, column: KeyValueColumn) -> Result<Vec<Value<'a>>>;

    fn get_column<'a>(&'a mut self, column: KeyValueColumn) -> Result<Vec<(Value<'a>, Value<'a>)>>;

    fn store(&mut self, column: KeyValueColumn, key: &[u8], value: &[u8]) -> Result<()>;

    fn delete(&mut self, column: KeyValueColumn, key: &[u8]) -> Result<()>;

    fn in_transaction(&self) -> bool;

    fn begin(&mut self) -> Result<()>;

    fn abort(&mut self) -> Result<()>;

    fn commit(&mut self) -> Result<()>;

    fn truncate(&mut self, column: KeyValueColumn) -> Result<()> {
        let keys = self
            .get_column_keys(column)?
            .into_iter()
            .map(|x| x.into_owned())
            .collect::<Vec<_>>();
        for key in keys {
            self.delete(column, &key[..])?;
        }
        Ok(())
    }
}
