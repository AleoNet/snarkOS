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
pub struct Iter<K, V> {
    db_iter: rocksdb::DBRawIterator,
    prefix: Vec<u8>,
    _phantom: PhantomData<(K, V)>,
}

impl<K: DeserializeOwned, V: DeserializeOwned> Iter<K, V> {
    pub(super) fn new(db_iter: rocksdb::DBRawIterator, prefix: Vec<u8>) -> Self {
        Self {
            db_iter,
            prefix,
            _phantom: PhantomData,
        }
    }
}

impl<K: DeserializeOwned, V: DeserializeOwned> Iterator for Iter<K, V> {
    type Item = (K, V);

    fn next(&mut self) -> Option<Self::Item> {
        if self.db_iter.valid() {
            let key = self.db_iter.key().and_then(|key| {
                if &key[0..self.prefix.len()] == &self.prefix[..] {
                    match bincode::deserialize(&key[self.prefix.len()..]) {
                        Ok(k) => k,
                        _ => None,
                    }
                } else {
                    None
                }
            });
            let value = match self.db_iter.value() {
                Some(value) => match bincode::deserialize(&value) {
                    Ok(v) => v,
                    _ => None,
                },
                None => None,
            };

            self.db_iter.next();
            key.and_then(|k| value.map(|v| (k, v)))
        } else {
            None
        }
    }
}
