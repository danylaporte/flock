use crate::iter::ManyIter;
use std::{
    iter::{Enumerate, FromIterator},
    marker::PhantomData,
    slice::Iter,
};

pub struct OneToMany<ONE, MANY> {
    _one: PhantomData<ONE>,
    vec: Vec<Vec<MANY>>,
}

impl<ONE, MANY> OneToMany<ONE, MANY> {
    pub fn contains_one_many(&self, id: ONE, many: MANY) -> bool
    where
        ONE: Into<usize>,
        MANY: Ord,
    {
        self.vec
            .get(id.into())
            .map_or(false, |vec| vec.binary_search(&many).is_ok())
    }

    pub fn iter(&self) -> OneIter<ONE, MANY> {
        OneIter {
            iter: self.vec.iter().enumerate(),
            _one: PhantomData,
        }
    }
    pub fn iter_by(&self, id: ONE) -> ManyIter<MANY>
    where
        ONE: Into<usize>,
        MANY: Clone,
    {
        ManyIter(
            self.vec
                .get(id.into())
                .map(|v| v.as_slice())
                .unwrap_or(&[])
                .into_iter(),
        )
    }
}

impl<ONE, MANY> FromIterator<(ONE, MANY)> for OneToMany<ONE, MANY>
where
    ONE: Into<usize>,
    MANY: Clone + Ord + PartialEq + Send,
{
    fn from_iter<T: IntoIterator<Item = (ONE, MANY)>>(iter: T) -> Self {
        let mut vec = Vec::new();

        for (one, many) in iter {
            let idx = one.into();
            vec.extend((vec.len()..=idx).into_iter().map(|_| Vec::new()));
            vec[idx].push(many);
        }

        for vec in &mut vec {
            vec.sort_unstable();
            vec.dedup();
            vec.shrink_to_fit();
        }

        vec.shrink_to_fit();

        Self {
            _one: PhantomData,
            vec,
        }
    }
}

pub struct OneIter<'a, ONE, MANY> {
    iter: Enumerate<Iter<'a, Vec<MANY>>>,
    _one: PhantomData<ONE>,
}

impl<'a, ONE, MANY> Clone for OneIter<'a, ONE, MANY> {
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            _one: PhantomData,
        }
    }
}

impl<'a, ONE, MANY> OneIter<'a, ONE, MANY>
where
    ONE: From<usize>,
{
    pub fn is_empty(mut self) -> bool {
        self.next().is_none()
    }
}

impl<'a, ONE, MANY> Iterator for OneIter<'a, ONE, MANY>
where
    ONE: From<usize>,
{
    type Item = (ONE, &'a [MANY]);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .filter(|(_, v)| !v.is_empty())
            .map(|(i, v)| (i.into(), v.as_slice()))
    }
}

#[test]
fn test_contains_one_many() {
    let o2m = std::iter::once((1, 3)).collect::<OneToMany<usize, usize>>();

    assert!(o2m.contains_one_many(1, 3));
    assert!(!o2m.contains_one_many(3, 1));
    assert!(!o2m.contains_one_many(2, 2));
    assert!(!o2m.contains_one_many(2, 5));
}
