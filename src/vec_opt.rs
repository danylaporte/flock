use std::{iter::FromIterator, mem::replace};

pub struct VecOpt<T> {
    vec: Vec<Option<T>>,
    len: usize,
}

impl<T> VecOpt<T> {
    pub fn clear(&mut self) {
        self.vec.iter_mut().for_each(|v| *v = None);
        self.len = 0;
    }

    fn ensure_index(&mut self, index: usize) {
        let empty_elements = (self.vec.len()..=index).into_iter().map(|_| None);
        self.vec.extend(empty_elements);
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        self.vec.get(idx)?.as_ref()
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.vec.get_mut(idx)?.as_mut()
    }

    pub fn insert(&mut self, index: usize, entity: T) {
        self.ensure_index(index);

        if replace(&mut self.vec[index], Some(entity)).is_none() {
            self.len += 1;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            iter: self.vec.iter(),
            len: self.len,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn remove(&mut self, idx: usize) -> Option<T> {
        if idx < self.vec.len() {
            let o = self.vec.get_mut(idx).and_then(|v| replace(v, None));

            if o.is_some() {
                self.len -= 1;
            }

            o
        } else {
            None
        }
    }

    pub(crate) fn remove_or_clear(&mut self, idx: Option<usize>) {
        match idx {
            Some(idx) => {
                self.remove(idx);
            }
            None => self.clear(),
        }
    }

    pub fn take(&mut self, index: usize) -> Option<T> {
        self.ensure_index(index);
        let item = replace(&mut self.vec[index], None);

        if item.is_some() {
            self.len -= 1;
        }

        item
    }
}

impl<T> Default for VecOpt<T> {
    fn default() -> Self {
        VecOpt {
            len: 0,
            vec: Vec::new(),
        }
    }
}

impl<K, V> FromIterator<(K, V)> for VecOpt<V>
where
    K: Into<usize>,
{
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut vec = VecOpt::default();

        for (key, value) in iter {
            vec.insert(key.into(), value);
        }

        vec
    }
}

pub struct Iter<'a, T> {
    iter: std::slice::Iter<'a, Option<T>>,
    len: usize,
}

impl<'a, T> Clone for Iter<'a, T> {
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            len: self.len.clone(),
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(item) = self.iter.next() {
            if let Some(item) = item.as_ref() {
                return Some(item);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect() {
        let v: VecOpt<_> = (0..2usize).map(|i| (i, false)).collect();
        assert_eq!(2, v.len());
    }

    #[test]
    fn insert_remove() {
        let mut v = VecOpt::default();

        v.insert(2, ());
        assert_eq!(v.len(), 1);
        assert_eq!(v.vec.len(), 3);

        v.insert(3, ());
        assert_eq!(v.len(), 2);
        assert_eq!(v.vec.len(), 4);

        v.insert(0, ());
        assert_eq!(v.len(), 3);
        assert_eq!(v.vec.len(), 4);

        v.remove(2);
        assert_eq!(v.len(), 2);
        assert_eq!(v.vec.len(), 4);
    }
}
