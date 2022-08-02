use super::*;

use std::borrow::Cow;

/// An iterator over the keys of a prefix.
pub struct Keys<'a, K: 'a + Debug + PartialEq + Eq + Hash + Serialize + DeserializeOwned> {
    db_iter: rocksdb::DBIterator<'a>,
    _phantom: PhantomData<K>,
}

impl<'a, K: 'a + Debug + PartialEq + Eq + Hash + Serialize + DeserializeOwned> Keys<'a, K> {
    pub(crate) fn new(db_iter: rocksdb::DBIterator<'a>) -> Self {
        Self {
            db_iter,
            _phantom: PhantomData,
        }
    }
}

impl<'a, K: 'a + Clone + Debug + PartialEq + Eq + Hash + Serialize + DeserializeOwned> Iterator for Keys<'a, K> {
    type Item = Cow<'a, K>;

    fn next(&mut self) -> Option<Self::Item> {
        let (key, _) = self.db_iter.next()?;
        let key = bincode::deserialize(&key[PREFIX_LEN..]).ok()?;

        Some(Cow::Owned(key))
    }
}
