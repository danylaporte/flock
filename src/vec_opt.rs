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

    pub fn get(&self, idx: usize) -> Option<&T> {
        self.vec.get(idx)?.as_ref()
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.vec.get_mut(idx)?.as_mut()
    }

    pub fn insert(&mut self, index: usize, entity: T) {
        let empty_elements = (self.vec.len()..=index).into_iter().map(|_| None);
        self.vec.extend(empty_elements);

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

    pub fn remove(&mut self, idx: usize) {
        if self
            .vec
            .get_mut(idx)
            .and_then(|v| replace(v, None))
            .is_some()
        {
            self.len -= 1;
        }
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

impl<A> FromIterator<(usize, A)> for VecOpt<A> {
    fn from_iter<I: IntoIterator<Item = (usize, A)>>(iter: I) -> Self {
        let mut vec = VecOpt::default();

        for (idx, item) in iter {
            vec.insert(idx, item);
        }

        vec
    }
}

pub struct Iter<'a, T> {
    iter: std::slice::Iter<'a, Option<T>>,
    len: usize,
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
        let v: VecOpt<_> = (0..2).map(|i| (i, false)).collect();
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
