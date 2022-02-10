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

/// An iterator over all key-value pairs in a data map.
pub struct Iter<'a, K, V> {
    db_iter: rocksdb::DBIterator<'a>,
    _phantom: PhantomData<(K, V)>,
}

impl<'a, K: DeserializeOwned, V: DeserializeOwned> Iter<'a, K, V> {
    pub(super) fn new(db_iter: rocksdb::DBIterator<'a>) -> Self {
        Self {
            db_iter,
            _phantom: PhantomData,
        }
    }
}

impl<'a, K: DeserializeOwned, V: DeserializeOwned> Iterator for Iter<'a, K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        let (key, value) = self.db_iter.next()?;
        let key = bincode::deserialize(&key[PREFIX_LEN..]).ok()?;
        let value = bincode::deserialize(&value).ok()?;

        Some((key, value))
    }
}
