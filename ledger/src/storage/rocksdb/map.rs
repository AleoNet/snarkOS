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

#[derive(Clone, Debug)]
pub struct DataMap<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned> {
    pub(super) rocksdb: Arc<rocksdb::DB>,
    pub(super) context: Vec<u8>,
    pub(super) _phantom: PhantomData<(K, V)>,
}

impl<'a, K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned> Map<'a, K, V> for DataMap<K, V> {
    type Iterator = Iter<'a, K, V>;
    type Keys = Keys<'a, K>;
    type Values = Values<'a, V>;

    ///
    /// Returns `true` if the given key exists in the map.
    ///
    fn contains_key<Q>(&self, key: &Q) -> Result<bool>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized,
    {
        self.get(key).map(|v| v.is_some())
    }

    ///
    /// Returns the value for the given key from the map, if it exists.
    ///
    fn get<Q>(&self, key: &Q) -> Result<Option<V>>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized,
    {
        let mut key_buf = self.context.clone();
        key_buf.reserve(bincode::serialized_size(&key)? as usize);
        bincode::serialize_into(&mut key_buf, &key)?;
        match self.rocksdb.get(&key_buf)? {
            Some(data) => Ok(Some(bincode::deserialize(&data)?)),
            None => Ok(None),
        }
    }

    ///
    /// Inserts the given key-value pair into the map.
    ///
    fn insert<Q>(&self, key: &Q, value: &V) -> Result<()>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized,
    {
        let mut key_buf = self.context.clone();
        key_buf.reserve(bincode::serialized_size(&key)? as usize);
        bincode::serialize_into(&mut key_buf, &key)?;
        let value_buf = bincode::serialize(value)?;

        self.rocksdb.put(&key_buf, &value_buf)?;
        Ok(())
    }

    ///
    /// Removes the key-value pair for the given key from the map.
    ///
    fn remove<Q>(&self, key: &Q) -> Result<()>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized,
    {
        let mut key_buf = self.context.clone();
        key_buf.reserve(bincode::serialized_size(&key)? as usize);
        bincode::serialize_into(&mut key_buf, &key)?;

        self.rocksdb.delete(&key_buf)?;
        Ok(())
    }

    ///
    /// Returns an iterator visiting each key-value pair in the map.
    ///
    fn iter(&'a self) -> Self::Iterator {
        let mut db_iter = self.rocksdb.raw_iterator();
        db_iter.seek(&self.context);

        Iter::new(db_iter, self.context.clone())
    }

    ///
    /// Returns an iterator over each key in the map.
    ///
    fn keys(&'a self) -> Self::Keys {
        let mut db_iter = self.rocksdb.raw_iterator();
        db_iter.seek(&self.context);

        Keys::new(db_iter, self.context.clone())
    }

    ///
    /// Returns an iterator over each value in the map.
    ///
    fn values(&'a self) -> Self::Values {
        let mut db_iter = self.rocksdb.raw_iterator();
        db_iter.seek(&self.context);

        Values::new(db_iter, self.context.clone())
    }
}
