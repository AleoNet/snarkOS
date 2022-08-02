// Copyright (C) 2019-2022 Aleo Systems Inc.
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

use super::*;

/// An iterator over the values of a prefix.
pub struct Values<'a, V> {
    db_iter: rocksdb::DBIterator<'a>,
    _phantom: PhantomData<V>,
}

impl<'a, V: DeserializeOwned> Values<'a, V> {
    pub(crate) fn new(db_iter: rocksdb::DBIterator<'a>) -> Self {
        Self {
            db_iter,
            _phantom: PhantomData,
        }
    }
}

impl<'a, V: DeserializeOwned> Iterator for Values<'a, V> {
    type Item = V;

    fn next(&mut self) -> Option<Self::Item> {
        let (_, value) = self.db_iter.next()?;
        let value = bincode::deserialize(&value).ok()?;

        Some(value)
    }
}
