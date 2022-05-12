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

pub const PREFIX_LEN: usize = 4; // N::NETWORK_ID (u16) + MapId (u16)

use anyhow::bail;
use rand::{thread_rng, Rng};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum MapId {
    BlockHeaders = 0,
    BlockHeights,
    BlockTransactions,
    Commitments,
    LedgerRoots,
    Records,
    SerialNumbers,
    Transactions,
    Transitions,
    Shares,
    #[cfg(test)]
    Test,
}

#[cfg(feature = "test")]
impl From<u16> for MapId {
    fn from(id: u16) -> Self {
        match id {
            0 => Self::BlockHeaders,
            1 => Self::BlockHeights,
            2 => Self::BlockTransactions,
            3 => Self::Commitments,
            4 => Self::LedgerRoots,
            5 => Self::Records,
            6 => Self::SerialNumbers,
            7 => Self::Transactions,
            8 => Self::Transitions,
            9 => Self::Shares,
            x => panic!("Unexpected map id: {}", x),
        }
    }
}

#[derive(Clone)]
pub struct DataMap<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned> {
    pub(super) storage: RocksDB,
    pub(super) context: Vec<u8>,
    pub(super) _phantom: PhantomData<(K, V)>,
}

impl<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned> DataMap<K, V> {
    #[inline]
    fn create_prefixed_key<Q>(&self, key: &Q) -> Result<Vec<u8>>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized,
    {
        let mut raw_key = self.context.clone();
        bincode::serialize_into(&mut raw_key, &key)?;

        Ok(raw_key)
    }

    fn get_raw<'a, Q>(&'a self, key: &Q) -> Result<Option<rocksdb::DBPinnableSlice<'a>>>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized,
    {
        let raw_key = self.create_prefixed_key(key)?;
        match self.storage.rocksdb.get_pinned(&raw_key)? {
            Some(data) => Ok(Some(data)),
            None => Ok(None),
        }
    }

    #[cfg(any(test, feature = "test"))]
    pub fn storage(&self) -> &RocksDB {
        &self.storage
    }
}

impl<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned> fmt::Debug for DataMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataMap").field("context", &self.context).finish()
    }
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
        self.get_raw(key).map(|v| v.is_some())
    }

    ///
    /// Returns the value for the given key from the map, if it exists.
    ///
    fn get<Q>(&self, key: &Q) -> Result<Option<V>>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized,
    {
        match self.get_raw(key) {
            Ok(Some(bytes)) => Ok(Some(bincode::deserialize(&bytes)?)),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    ///
    /// Inserts the given key-value pair into the map. Can be paired with a numeric
    /// batch id, which defers the operation until `execute_batch` is called using
    /// the same id.
    ///
    fn insert<Q>(&self, key: &Q, value: &V, batch: Option<usize>) -> Result<()>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized,
    {
        let raw_key = self.create_prefixed_key(key)?;
        let raw_value = bincode::serialize(value)?;

        if let Some(batch_id) = batch {
            self.storage.batches.lock().entry(batch_id).or_default().put(&raw_key, &raw_value);
        } else {
            self.storage.rocksdb.put(&raw_key, &raw_value)?;
        }

        Ok(())
    }

    ///
    /// Removes the key-value pair for the given key from the map. Can be paired with a
    /// numeric batch id, which defers the operation until `execute_batch` is called using
    /// the same id.
    ///
    fn remove<Q>(&self, key: &Q, batch: Option<usize>) -> Result<()>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized,
    {
        let raw_key = self.create_prefixed_key(key)?;

        if let Some(batch_id) = batch {
            self.storage.batches.lock().entry(batch_id).or_default().delete(&raw_key);
        } else {
            self.storage.rocksdb.delete(&raw_key)?;
        }

        Ok(())
    }

    ///
    /// Returns an iterator visiting each key-value pair in the map.
    ///
    fn iter(&'a self) -> Self::Iterator {
        Iter::new(self.storage.rocksdb.prefix_iterator(&self.context))
    }

    ///
    /// Returns an iterator over each key in the map.
    ///
    fn keys(&'a self) -> Self::Keys {
        Keys::new(self.storage.rocksdb.prefix_iterator(&self.context))
    }

    ///
    /// Returns an iterator over each value in the map.
    ///
    fn values(&'a self) -> Self::Values {
        Values::new(self.storage.rocksdb.prefix_iterator(&self.context))
    }

    ///
    /// Performs a refresh operation for implementations of `Map` that perform periodic operations.
    /// This method is implemented here for RocksDB to catch up a reader (secondary) database.
    /// Returns `true` if the sequence number of the database has increased.
    ///
    fn refresh(&self) -> bool {
        // If the storage is in read-only mode, catch it up to its writable storage.
        if self.storage.is_read_only {
            let original_sequence_number = self.storage.rocksdb.latest_sequence_number();
            if self.storage.rocksdb.try_catch_up_with_primary().is_ok() {
                let new_sequence_number = self.storage.rocksdb.latest_sequence_number();
                return new_sequence_number > original_sequence_number;
            }
        }
        false
    }

    ///
    /// Prepares an atomic batch of writes and returns its numeric id which can later be used to include
    /// operations within it. `execute_batch` has to be called in order for any of the writes to actually
    /// take place.
    ///
    fn prepare_batch(&self) -> usize {
        let mut id = thread_rng().gen();

        while self.storage.batches.lock().contains_key(&id) {
            id = thread_rng().gen();
        }

        id
    }

    ///
    /// Atomically executes a write batch with the given id.
    ///
    fn execute_batch(&self, batch: usize) -> Result<()> {
        if let Some(batch) = self.storage.batches.lock().remove(&batch) {
            Ok(self.storage.rocksdb.write(batch)?)
        } else {
            bail!("There is no pending storage batch with id = {}", batch);
        }
    }

    ///
    /// Discards a write batch with the given id.
    ///
    fn discard_batch(&self, batch: usize) -> Result<()> {
        if self.storage.batches.lock().remove(&batch).is_none() {
            bail!("Attempted to discard a non-existent storage batch (id = {})", batch)
        } else {
            Ok(())
        }
    }
}
