use super::*;

use snarkvm::compiler::{Map, MapReader};

use core::{
    fmt::{self, Debug},
    hash::Hash,
};
use std::borrow::Cow;

#[derive(Clone)]
pub struct DataMap<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned> {
    pub(super) storage: RocksDB,
    pub(super) context: Vec<u8>,
    pub(super) _phantom: PhantomData<(K, V)>,
}

impl<
    'a,
    K: 'a + Clone + Debug + PartialEq + Eq + Hash + Serialize + DeserializeOwned,
    V: 'a + Clone + PartialEq + Eq + Serialize + DeserializeOwned,
> Map<'a, K, V> for DataMap<K, V>
{
    ///
    /// Inserts the given key-value pair into the map.
    ///
    fn insert(&mut self, key: K, value: V) -> Result<()> {
        let raw_key = self.create_prefixed_key(&key)?;
        let raw_value = bincode::serialize(&value)?;
        self.storage.rocksdb.put(&raw_key, &raw_value)?;

        Ok(())
    }

    ///
    /// Removes the key-value pair for the given key from the map.
    ///
    fn remove<Q>(&mut self, key: &Q) -> Result<()>
    where
        K: Borrow<Q>,
        Q: PartialEq + Eq + Hash + Serialize + ?Sized,
    {
        let raw_key = self.create_prefixed_key(key)?;
        self.storage.rocksdb.delete(&raw_key)?;

        Ok(())
    }
}

impl<
    'a,
    K: 'a + Clone + Debug + PartialEq + Eq + Hash + Serialize + DeserializeOwned,
    V: 'a + Clone + PartialEq + Eq + Serialize + DeserializeOwned,
> MapReader<'a, K, V> for DataMap<K, V>
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
}

impl<
    'a,
    K: 'a + Clone + PartialEq + Eq + Hash + Serialize + DeserializeOwned,
    V: 'a + Clone + PartialEq + Eq + Serialize + DeserializeOwned,
> FromIterator<(K, V)> for DataMap<K, V>
{
    /// Initializes a new `DataMap` from the given iterator.
    fn from_iter<I: IntoIterator<Item = (K, V)>>(_iter: I) -> Self {
        unimplemented!()
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
        match self.storage.rocksdb.get_pinned(&raw_key)? {
            Some(data) => Ok(Some(data)),
            None => Ok(None),
        }
    }

    pub fn storage(&self) -> &RocksDB {
        &self.storage
    }
}

impl<K: Serialize + DeserializeOwned, V: Serialize + DeserializeOwned> fmt::Debug for DataMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataMap").field("context", &self.context).finish()
    }
}
