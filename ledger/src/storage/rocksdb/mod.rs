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

mod iterator;
use iterator::*;

mod keys;
use keys::*;

mod map;
pub(crate) use map::*;

mod values;
use values::*;

#[cfg(test)]
mod tests;

use crate::storage::{Map, Storage};

use anyhow::Result;
use serde::{
    de::{self, DeserializeOwned},
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
    fn open<P: AsRef<Path>>(path: P, context: u16, is_read_only: bool) -> Result<Self> {
        let context = context.to_le_bytes();
        let mut context_bytes = bincode::serialize(&(context.len() as u32)).unwrap();
        context_bytes.extend_from_slice(&context);

        let rocksdb = match is_read_only {
            true => Arc::new(rocksdb::DB::open_for_read_only(&rocksdb::Options::default(), path, false)?),
            false => Arc::new(rocksdb::DB::open_default(path)?),
        };

        Ok(RocksDB {
            rocksdb,
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

        impl<'de> de::Visitor<'de> for RocksDBVisitor {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a rocksdb seq")
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut map: A) -> std::result::Result<(), A::Error> {
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
