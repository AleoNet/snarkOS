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

pub(crate) mod iterator;
pub(crate) use iterator::*;

pub(crate) mod keys;
pub(crate) use keys::*;

pub(crate) mod values;
pub(crate) use values::*;

use crate::storage::{Map, Storage};

use anyhow::Result;
use serde::{
    de::{DeserializeOwned, SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserializer,
    Serialize,
    Serializer,
};
use std::{borrow::Borrow, fmt, marker::PhantomData, path::Path, sync::Arc};

///
/// An instance of a RocksDB database.
///
#[derive(Clone)]
pub struct RocksDB {
    rocksdb: Arc<rocksdb::DB>,
    context: Vec<u8>,
}

impl Storage for RocksDB {
    ///
    /// Opens storage at the given `path` and `context`.
    ///
    fn open<P: AsRef<Path>>(path: P, context: u16) -> Result<Self> {
        let context = context.to_le_bytes();
        let mut context_bytes = bincode::serialize(&(context.len() as u32)).unwrap();
        context_bytes.extend_from_slice(&context);

        Ok(RocksDB {
            rocksdb: Arc::new(rocksdb::DB::open_default(path)?),
            context: context_bytes,
        })
    }

    ///
    /// Opens a map with the given `context` from storage.
    ///
    fn open_map<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned>(&self, context: &str) -> Result<DataMap<K, V>> {
        // Convert the new context into bytes.
        let new_context = context.as_bytes();

        // Combine contexts to create a new scope.
        let mut context_bytes = self.context.clone();
        bincode::serialize_into(&mut context_bytes, &(new_context.len() as u32))?;
        context_bytes.extend_from_slice(&new_context);

        Ok(DataMap {
            rocksdb: self.rocksdb.clone(),
            context: context_bytes,
            _phantom: PhantomData,
        })
    }

    ///
    /// Imports the given serialized bytes to reconstruct storage.
    ///
    fn import<'de, D: Deserializer<'de>>(&self, deserializer: D) -> Result<(), D::Error> {
        struct RocksDBVisitor {
            rocksdb: RocksDB,
        }

        impl<'de> Visitor<'de> for RocksDBVisitor {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a rocksdb seq")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut map: A) -> std::result::Result<(), A::Error> {
                while let Some((key, value)) = map.next_element::<(Vec<_>, Vec<_>)>()? {
                    self.rocksdb.rocksdb.put(&key, &value).map_err(|e| serde::de::Error::custom(e))?;
                }

                Ok(())
            }
        }

        deserializer.deserialize_seq(RocksDBVisitor { rocksdb: self.clone() })?;

        Ok(())
    }

    ///
    /// Exports the current state of storage into serialized bytes.
    ///
    fn export(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(&self)?)
    }
}

impl Serialize for RocksDB {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut iterator = self.rocksdb.raw_iterator();
        iterator.seek_to_first();

        let mut map = serializer.serialize_seq(None)?;
        while iterator.valid() {
            if let (Some(key), Some(value)) = (iterator.key(), iterator.value()) {
                map.serialize_element(&(key, value))?;
            }
            iterator.next();
        }
        map.end()
    }
}

pub struct DataMap<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned> {
    rocksdb: Arc<rocksdb::DB>,
    context: Vec<u8>,
    _phantom: PhantomData<(K, V)>,
}

impl<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned> Map<K, V> for DataMap<K, V> {
    type Iterator = Iter<K, V>;
    type Keys = Keys<K>;
    type Values = Values<V>;

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
    fn iter(&self) -> Self::Iterator {
        let mut db_iter = self.rocksdb.raw_iterator();
        db_iter.seek(&self.context);

        Iter::new(db_iter, self.context.clone())
    }

    ///
    /// Returns an iterator over each key in the map.
    ///
    fn keys(&self) -> Self::Keys {
        let mut db_iter = self.rocksdb.raw_iterator();
        db_iter.seek(&self.context);

        Keys::new(db_iter, self.context.clone())
    }

    ///
    /// Returns an iterator over each value in the map.
    ///
    fn values(&self) -> Self::Values {
        let mut db_iter = self.rocksdb.raw_iterator();
        db_iter.seek(&self.context);

        Values::new(db_iter, self.context.clone())
    }
}
