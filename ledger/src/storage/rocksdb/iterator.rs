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

use super::*;

/// An iterator over all key-value pairs in a data map.
pub struct Iter<'a, K, V> {
    db_iter: rocksdb::DBRawIterator<'a>,
    prefix: Vec<u8>,
    _phantom: PhantomData<(K, V)>,
}

impl<'a, K: DeserializeOwned, V: DeserializeOwned> Iter<'a, K, V> {
    pub(super) fn new(db_iter: rocksdb::DBRawIterator<'a>, prefix: Vec<u8>) -> Self {
        Self {
            db_iter,
            prefix,
            _phantom: PhantomData,
        }
    }
}

impl<'a, K: DeserializeOwned, V: DeserializeOwned> Iterator for Iter<'a, K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.db_iter.valid() {
            let key = match self
                .db_iter
                .key()
                .and_then(|k| {
                    if &k[0..self.prefix.len()] == &self.prefix[..] {
                        Some(k)
                    } else {
                        None
                    }
                })
                .map(|k| bincode::deserialize(&k[self.prefix.len()..]).ok())
            {
                Some(key) => key,
                None => None,
            };
            let value = match self.db_iter.value().map(|v| bincode::deserialize(&v).ok()) {
                Some(value) => value,
                None => None,
            };

            self.db_iter.next();
            key.and_then(|k| value.map(|v| (k, v)))
        } else {
            None
        }
    }
}
