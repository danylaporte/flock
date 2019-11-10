use std::{marker::PhantomData, slice::Iter as SliceIter};
use version_tag::VersionTag;

pub struct IndexMap<K, V> {
    _k: PhantomData<K>,
    len: usize,
    tag: VersionTag,
    vec: Vec<Option<V>>,
}

impl<K, V> IndexMap<K, V> {
    pub fn clear(&mut self) {
        self.vec.clear();
        self.len = 0;
    }

    pub fn get(&self, key: K) -> Option<&V>
    where
        K: Into<usize>,
    {
        self.vec.get(key.into()).and_then(|entity| entity.as_ref())
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> Iter<K, V>
    where
        K: From<usize>,
    {
        Iter {
            _k: PhantomData,
            idx: 0,
            it: self.vec.iter(),
            len: self.len,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn tag(&self) -> VersionTag {
        self.tag
    }
}

impl<'a, K, V> IntoIterator for &'a IndexMap<K, V>
where
    K: From<usize>,
{
    type Item = (K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct Iter<'a, K, V> {
    _k: PhantomData<K>,
    idx: usize,
    it: SliceIter<'a, Option<V>>,
    len: usize,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: From<usize>,
{
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.it.next() {
            if let Some(item) = item {
                self.idx += 1;
                return Some((K::from(self.idx - 1), item));
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }

    fn count(self) -> usize {
        self.len
    }
}
