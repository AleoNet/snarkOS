use super::*;

use std::borrow::Cow;

/// An iterator over the values of a prefix.
pub struct Values<'a, V: 'a + PartialEq + Eq + Serialize + DeserializeOwned + Sync> {
    db_iter: rocksdb::DBIterator<'a>,
    _phantom: PhantomData<V>,
}

impl<'a, V: 'a + PartialEq + Eq + Serialize + DeserializeOwned + Sync> Values<'a, V> {
    pub(crate) fn new(db_iter: rocksdb::DBIterator<'a>) -> Self {
        Self {
            db_iter,
            _phantom: PhantomData,
        }
    }
}

impl<'a, V: 'a + Clone + PartialEq + Eq + Serialize + DeserializeOwned + Sync> Iterator for Values<'a, V> {
    type Item = Cow<'a, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let (_, value) = self.db_iter.next()?;
        let value = bincode::deserialize(&value).ok()?;

        Some(Cow::Owned(value))
    }
}
