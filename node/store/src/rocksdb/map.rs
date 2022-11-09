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

use indexmap::IndexMap;
use rocksdb::WriteBatch;
use snarkvm::synthesizer::store::helpers::{Map, MapRead};

use core::{fmt, fmt::Debug, hash::Hash};
use std::{borrow::Cow, sync::atomic::Ordering};

#[derive(Clone)]
pub struct DataMap<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned> {
    pub(super) database: RocksDB,
    pub(super) context: Vec<u8>,
    /// The tracker for whether a database transaction is in progress.
    pub(super) batch_in_progress: Arc<AtomicBool>,
    /// The database transaction.
    pub(super) atomic_batch: Arc<Mutex<IndexMap<K, Option<V>>>>,
}

impl<
    'a,
    K: 'a + Copy + Clone + Debug + PartialEq + Eq + Hash + Serialize + DeserializeOwned + Send + Sync,
    V: 'a + Clone + PartialEq + Eq + Serialize + DeserializeOwned + Send + Sync,
> Map<'a, K, V> for DataMap<K, V>
{
    ///
    /// Inserts the given key-value pair into the map.
    ///
    fn insert(&self, key: K, value: V) -> Result<()> {
        // Determine if an atomic batch is in progress.
        let is_batch = self.batch_in_progress.load(Ordering::SeqCst);

        match is_batch {
            // If a batch is in progress, add the key-value pair to the batch.
            true => {
                self.atomic_batch.lock().insert(key, Some(value));
            }
            // Otherwise, insert the key-value pair directly into the map.
            false => {
                // Prepare the prefixed key and serialized value.
                let raw_key = self.create_prefixed_key(&key)?;
                let raw_value = bincode::serialize(&value)?;
                self.database.put(raw_key, raw_value)?;
            }
        }

        Ok(())
    }

    ///
    /// Removes the key-value pair for the given key from the map.
    ///
    fn remove(&self, key: &K) -> Result<()> {
        // Determine if an atomic batch is in progress.
        let is_batch = self.batch_in_progress.load(Ordering::SeqCst);

        match is_batch {
            // If a batch is in progress, add the key to the batch.
            true => {
                self.atomic_batch.lock().insert(*key, None);
            }
            // Otherwise, remove the key-value pair directly from the map.
            false => {
                // Prepare the prefixed key.
                let raw_key = self.create_prefixed_key(key)?;
                self.database.delete(raw_key)?;
            }
        }

        Ok(())
    }

    ///
    /// Begins an atomic operation. Any further calls to `insert` and `remove` will be queued
    /// without an actual write taking place until `finish_atomic` is called.
    ///
    fn start_atomic(&self) {
        // Set the atomic batch flag to `true`.
        self.batch_in_progress.store(true, Ordering::SeqCst);
        // Ensure that the atomic batch is empty.
        assert!(self.atomic_batch.lock().is_empty());
    }

    ///
    /// Checks whether an atomic operation is currently in progress. This can be done to ensure
    /// that lower-level operations don't start and finish their individual atomic write batch
    /// if they are already part of a larger one.
    ///
    fn is_atomic_in_progress(&self) -> bool {
        self.batch_in_progress.load(Ordering::SeqCst)
    }

    ///
    /// Aborts the current atomic operation.
    ///
    fn abort_atomic(&self) {
        // Clear the atomic batch.
        *self.atomic_batch.lock() = Default::default();
        // Set the atomic batch flag to `false`.
        self.batch_in_progress.store(false, Ordering::SeqCst);
    }

    ///
    /// Finishes an atomic operation, performing all the queued writes.
    ///
    fn finish_atomic(&self) -> Result<()> {
        // Retrieve the atomic batch.
        let operations = core::mem::take(&mut *self.atomic_batch.lock());

        if !operations.is_empty() {
            // Prepare operations batch for underlying database.
            let mut batch = WriteBatch::default();
            for operation in operations {
                match operation {
                    (key, Some(value)) => {
                        // Prepare the prefixed key and serialized value for insertion.
                        let raw_key = self.create_prefixed_key(&key)?;
                        let raw_value = bincode::serialize(&value)?;
                        batch.put(raw_key, raw_value);
                    }
                    (key, None) => {
                        // Prepare the prefixed key for deletion.
                        let raw_key = self.create_prefixed_key(&key)?;
                        batch.delete(raw_key);
                    }
                };
            }
            // Execute all the operations atomically.
            self.database.rocksdb.write(batch)?;
        }

        // Set the atomic batch flag to `false`.
        self.batch_in_progress.store(false, Ordering::SeqCst);

        Ok(())
    }
}

impl<
    'a,
    K: 'a + Copy + Clone + Debug + PartialEq + Eq + Hash + Serialize + DeserializeOwned + Send + Sync,
    V: 'a + Clone + PartialEq + Eq + Serialize + DeserializeOwned + Send + Sync,
> MapRead<'a, K, V> for DataMap<K, V>
{
    type Iterator = Iter<'a, K, V>;
    type Keys = Keys<'a, K>;
    type Values = Values<'a, V>;

    ///
    /// Returns `true` if the given key exists in the map.
    ///
    fn contains_key<Q>(&self, key: &Q) -> Result<bool>
    where
        K: Borrow<Q>,
        Q: PartialEq + Eq + Hash + Serialize + ?Sized,
    {
        self.get_raw(key).map(|v| v.is_some())
    }

    ///
    /// Returns the value for the given key from the map, if it exists.
    ///
    fn get<Q>(&'a self, key: &Q) -> Result<Option<Cow<'a, V>>>
    where
        K: Borrow<Q>,
        Q: PartialEq + Eq + Hash + Serialize + ?Sized,
    {
        match self.get_raw(key) {
            Ok(Some(bytes)) => Ok(Some(Cow::Owned(bincode::deserialize(&bytes)?))),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }

    ///
    /// Returns the current value for the given key if it is scheduled
    /// to be inserted as part of an atomic batch.
    ///
    /// If the key does not exist, returns `None`.
    /// If the key is removed in the batch, returns `Some(None)`.
    /// If the key is inserted in the batch, returns `Some(Some(value))`.
    ///
    fn get_batched<Q>(&self, key: &Q) -> Option<Option<V>>
    where
        K: Borrow<Q>,
        Q: PartialEq + Eq + Hash + Serialize + ?Sized,
    {
        if self.batch_in_progress.load(Ordering::SeqCst) { self.atomic_batch.lock().get(key).cloned() } else { None }
    }

    ///
    /// Returns an iterator visiting each key-value pair in the map.
    ///
    fn iter(&'a self) -> Self::Iterator {
        Iter::new(self.database.prefix_iterator(&self.context))
    }

    ///
    /// Returns an iterator over each key in the map.
    ///
    fn keys(&'a self) -> Self::Keys {
        Keys::new(self.database.prefix_iterator(&self.context))
    }

    ///
    /// Returns an iterator over each value in the map.
    ///
    fn values(&'a self) -> Self::Values {
        Values::new(self.database.prefix_iterator(&self.context))
    }
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

    fn get_raw<Q>(&self, key: &Q) -> Result<Option<rocksdb::DBPinnableSlice>>
    where
        K: Borrow<Q>,
        Q: Serialize + ?Sized,
    {
        let raw_key = self.create_prefixed_key(key)?;
        match self.database.get_pinned(&raw_key)? {
            Some(data) => Ok(Some(data)),
            None => Ok(None),
        }
    }
}

impl<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned> fmt::Debug for DataMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataMap").field("context", &self.context).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rocksdb::tests::temp_dir;
    use snarkvm::prelude::{Address, FromStr, Testnet3};

    use serial_test::serial;
    use tracing_test::traced_test;

    type CurrentNetwork = Testnet3;

    #[test]
    #[serial]
    fn test_contains_key() {
        // Initialize an address.
        let address =
            Address::<CurrentNetwork>::from_str("aleo1q6qstg8q8shwqf5m6q5fcenuwsdqsvp4hhsgfnx5chzjm3secyzqt9mxm8")
                .unwrap();

        // Initialize a map.
        let map: DataMap<Address<CurrentNetwork>, ()> =
            RocksDB::open_map_testing(temp_dir(), None, DataID::Test).expect("Failed to open data map");
        map.insert(address, ()).expect("Failed to insert into data map");
        assert!(map.contains_key(&address).unwrap());
    }

    #[test]
    #[serial]
    #[traced_test]
    fn test_insert_and_get_speculative() {
        // Initialize a map.
        let map: DataMap<usize, String> =
            RocksDB::open_map_testing(temp_dir(), None, DataID::Test).expect("Failed to open data map");

        // Sanity check.
        assert!(map.iter().next().is_none());

        /* test atomic insertions */

        // Start an atomic write batch.
        map.start_atomic();

        // Insert an item into the map.
        map.insert(0, "0".to_string()).unwrap();

        // Check that the item is not yet in the map.
        assert!(map.get(&0).unwrap().is_none());
        // Check that the item is in the batch.
        assert_eq!(map.get_batched(&0), Some(Some("0".to_string())));
        // Check that the item can be speculatively retrieved.
        assert_eq!(map.get_speculative(&0).unwrap(), Some(Cow::Owned("0".to_string())));

        // Queue (since a batch is in progress) NUM_ITEMS insertions.
        for i in 1..10 {
            // Update the item in the map.
            map.insert(0, i.to_string()).unwrap();

            // Check that the item is not yet in the map.
            assert!(map.get(&0).unwrap().is_none());
            // Check that the updated item is in the batch.
            assert_eq!(map.get_batched(&0), Some(Some(i.to_string())));
            // Check that the updated item can be speculatively retrieved.
            assert_eq!(map.get_speculative(&0).unwrap(), Some(Cow::Owned(i.to_string())));
        }

        // The map should still contain no items.
        assert!(map.iter().next().is_none());

        // Finish the current atomic write batch.
        map.finish_atomic().unwrap();

        // Check that the item is present in the map now.
        assert_eq!(map.get(&0).unwrap(), Some(Cow::Owned("9".to_string())));
        // Check that the item is not in the batch.
        assert_eq!(map.get_batched(&0), None);
        // Check that the item can be speculatively retrieved.
        assert_eq!(map.get_speculative(&0).unwrap(), Some(Cow::Owned("9".to_string())));
    }

    #[test]
    #[serial]
    #[traced_test]
    fn test_remove_and_get_speculative() {
        // Initialize a map.
        let map: DataMap<usize, String> =
            RocksDB::open_map_testing(temp_dir(), None, DataID::Test).expect("Failed to open data map");

        // Sanity check.
        assert!(map.iter().next().is_none());

        // Insert an item into the map.
        map.insert(0, "0".to_string()).unwrap();

        // Check that the item is present in the map .
        assert_eq!(map.get(&0).unwrap(), Some(Cow::Owned("0".to_string())));
        // Check that the item is not in the batch.
        assert_eq!(map.get_batched(&0), None);
        // Check that the item can be speculatively retrieved.
        assert_eq!(map.get_speculative(&0).unwrap(), Some(Cow::Owned("0".to_string())));

        /* test atomic removals */

        // Start an atomic write batch.
        map.start_atomic();

        // Remove the item from the map.
        map.remove(&0).unwrap();

        // Check that the item still exists in the map.
        assert_eq!(map.get(&0).unwrap(), Some(Cow::Owned("0".to_string())));
        // Check that the item is removed in the batch.
        assert_eq!(map.get_batched(&0), Some(None));
        // Check that the item is removed when speculatively retrieved.
        assert_eq!(map.get_speculative(&0).unwrap(), None);

        // Try removing the item again.
        map.remove(&0).unwrap();

        // Check that the item still exists in the map.
        assert_eq!(map.get(&0).unwrap(), Some(Cow::Owned("0".to_string())));
        // Check that the item is removed in the batch.
        assert_eq!(map.get_batched(&0), Some(None));
        // Check that the item is removed when speculatively retrieved.
        assert_eq!(map.get_speculative(&0).unwrap(), None);

        // Finish the current atomic write batch.
        map.finish_atomic().unwrap();

        // Check that the item is not present in the map now.
        assert!(map.get(&0).unwrap().is_none());
        // Check that the item is not in the batch.
        assert_eq!(map.get_batched(&0), None);
        // Check that the item is removed when speculatively retrieved.
        assert_eq!(map.get_speculative(&0).unwrap(), None);

        // Check that the map is empty now.
        assert!(map.iter().next().is_none());
    }

    #[test]
    #[serial]
    #[traced_test]
    fn test_atomic_writes_are_batched() {
        // The number of items that will be inserted into the map.
        const NUM_ITEMS: usize = 10;

        // Initialize a map.
        let map: DataMap<usize, String> =
            RocksDB::open_map_testing(temp_dir(), None, DataID::Test).expect("Failed to open data map");

        // Sanity check.
        assert!(map.iter().next().is_none());

        /* test atomic insertions */

        // Start an atomic write batch.
        map.start_atomic();

        // Queue (since a batch is in progress) NUM_ITEMS insertions.
        for i in 0..NUM_ITEMS {
            map.insert(i, i.to_string()).unwrap();
            // Ensure that the item is queued for insertion.
            assert_eq!(map.get_batched(&i), Some(Some(i.to_string())));
            // Ensure that the item can be found with a speculative get.
            assert_eq!(map.get_speculative(&i).unwrap(), Some(Cow::Owned(i.to_string())));
        }

        // The map should still contain no items.
        assert!(map.iter().next().is_none());

        // Finish the current atomic write batch.
        map.finish_atomic().unwrap();

        // Check that the items are present in the map now.
        for i in 0..NUM_ITEMS {
            assert_eq!(map.get(&i).unwrap(), Some(Cow::Borrowed(&i.to_string())));
        }

        /* test atomic removals */

        // Start an atomic write batch.
        map.start_atomic();

        // Queue (since a batch is in progress) NUM_ITEMS removals.
        for i in 0..NUM_ITEMS {
            map.remove(&i).unwrap();
            // Ensure that the item is NOT queued for insertion.
            assert_eq!(map.get_batched(&i), Some(None));
        }

        // The map should still contains all the items.
        assert_eq!(map.iter().count(), NUM_ITEMS);

        // Finish the current atomic write batch.
        map.finish_atomic().unwrap();

        // Check that the map is empty now.
        assert!(map.iter().next().is_none());
    }

    #[test]
    #[serial]
    #[traced_test]
    fn test_atomic_writes_can_be_aborted() {
        // The number of items that will be queued to be inserted into the map.
        const NUM_ITEMS: usize = 10;

        // Initialize a map.
        let map: DataMap<usize, String> =
            RocksDB::open_map_testing(temp_dir(), None, DataID::Test).expect("Failed to open data map");

        // Sanity check.
        assert!(map.iter().next().is_none());

        // Start an atomic write batch.
        map.start_atomic();

        // Queue (since a batch is in progress) NUM_ITEMS insertions.
        for i in 0..NUM_ITEMS {
            map.insert(i, i.to_string()).unwrap();
        }

        // The map should still contain no items.
        assert!(map.iter().next().is_none());

        // Abort the current atomic write batch.
        map.abort_atomic();

        // The map should still contain no items.
        assert!(map.iter().next().is_none());

        // Start another atomic write batch.
        map.start_atomic();

        // Queue (since a batch is in progress) NUM_ITEMS insertions.
        for i in 0..NUM_ITEMS {
            map.insert(i, i.to_string()).unwrap();
        }

        // Finish the current atomic write batch.
        map.finish_atomic().unwrap();

        // The map should contain NUM_ITEMS items now.
        assert_eq!(map.iter().count(), NUM_ITEMS);
    }
}
