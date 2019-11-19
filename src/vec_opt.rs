use std::mem::replace;

#[doc(hidden)]
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

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.vec.iter().filter_map(|i| i.as_ref())
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

#[cfg(test)]
mod tests {
    use super::*;

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
