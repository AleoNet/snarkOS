use super::*;

use std::borrow::Cow;

/// An iterator over all key-value pairs in a data map.
pub struct Iter<
    'a,
    K: 'a + Debug + PartialEq + Eq + Hash + Serialize + DeserializeOwned,
    V: 'a + PartialEq + Eq + Serialize + DeserializeOwned,
> {
    db_iter: rocksdb::DBIterator<'a>,
    _phantom: PhantomData<(K, V)>,
}

impl<'a, K: 'a + Debug + PartialEq + Eq + Hash + Serialize + DeserializeOwned, V: 'a + PartialEq + Eq + Serialize + DeserializeOwned>
    Iter<'a, K, V>
{
    pub(super) fn new(db_iter: rocksdb::DBIterator<'a>) -> Self {
        Self {
            db_iter,
            _phantom: PhantomData,
        }
    }
}

impl<
    'a,
    K: 'a + Clone + Debug + PartialEq + Eq + Hash + Serialize + DeserializeOwned,
    V: 'a + Clone + PartialEq + Eq + Serialize + DeserializeOwned,
> Iterator for Iter<'a, K, V>
{
    type Item = (Cow<'a, K>, Cow<'a, V>);

    fn next(&mut self) -> Option<Self::Item> {
        let (key, value) = self.db_iter.next()?;
        let key = bincode::deserialize(&key[PREFIX_LEN..]).ok()?;
        let value = bincode::deserialize(&value).ok()?;

        Some((Cow::Owned(key), Cow::Owned(value)))
    }
}
