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

use super::{DataMap, MapId};

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};
use std::{borrow::Borrow, path::Path};

pub trait Storage {
    ///
    /// Opens storage at the given `path` and `context`.
    ///
    fn open<P: AsRef<Path>>(path: P, context: u16, is_read_only: bool) -> Result<Self>
    where
        Self: Sized;

    ///
    /// Opens a map with the given `context` from storage.
    ///
    fn open_map<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned>(&self, map_id: MapId) -> Result<DataMap<K, V>>;

    ///
    /// Imports a file with the given path to reconstruct storage.
    ///
    fn import<P: AsRef<Path>>(&self, path: P) -> Result<()>;

    ///
    /// Exports the current state of storage to a single file at the specified location.
    ///
    fn export<P: AsRef<Path>>(&self, path: P) -> Result<()>;
}

pub trait Map<'a, K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned> {
    type Iterator: Iterator<Item = (K, V)>;
    type Keys: Iterator<Item = K>;
    type Values: Iterator<Item = V>;

    ///
    /// Returns `true` if the given key exists in the map.
    ///
    fn contains_key<Q>(&self, key: &Q) -> Result<bool>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized;

    ///
    /// Returns the value for the given key from the map, if it exists.
    ///
    fn get<Q>(&self, key: &Q) -> Result<Option<V>>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized;

    ///
    /// Inserts the given key-value pair into the map. Can be paired with a numeric
    /// batch id, which defers the operation until `execute_batch` is called using
    /// the same id.
    ///
    fn insert<Q>(&self, key: &Q, value: &V, batch: Option<usize>) -> Result<()>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized;

    ///
    /// Removes the key-value pair for the given key from the map. Can be paired with a
    /// numeric batch id, which defers the operation until `execute_batch` is called using
    /// the same id.
    ///
    fn remove<Q>(&self, key: &Q, batch: Option<usize>) -> Result<()>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized;

    ///
    /// Returns an iterator visiting each key-value pair in the map.
    ///
    fn iter(&'a self) -> Self::Iterator;

    ///
    /// Returns an iterator over each key in the map.
    ///
    fn keys(&'a self) -> Self::Keys;

    ///
    /// Returns an iterator over each value in the map.
    ///
    fn values(&'a self) -> Self::Values;

    ///
    /// Performs a refresh operation for implementations of `Map` that perform periodic operations.
    /// Returns `true` if the database state has been updated.
    ///
    fn refresh(&self) -> bool {
        // Currently, this method is implemented for RocksDB to catch up a reader (secondary) database.
        true
    }

    ///
    /// Prepares an atomic batch of writes and returns its numeric id which can later be used to include
    /// operations within it. `execute_batch` has to be called in order for any of the writes to actually
    /// take place.
    ///
    fn prepare_batch(&self) -> usize;

    ///
    /// Atomically executes a write batch with the given id.
    ///
    fn execute_batch(&self, batch: usize) -> Result<()>;

    ///
    /// Discards a write batch with the given id.
    ///
    fn discard_batch(&self, batch: usize) -> Result<()>;
}
