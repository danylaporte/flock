use crate::EntityBy;
use std::{marker::PhantomData, slice::Iter};

pub trait FlockIter: Iterator {
    fn entities<'a, K, V, E>(self, entity_by: &'a E) -> EntityIter<'a, E, Self, V>
    where
        Self: Sized,
        E: EntityBy<K, V>,
    {
        EntityIter {
            _v: PhantomData,
            entity_by,
            iter: self,
        }
    }
}

impl<T> FlockIter for T where T: Iterator {}

pub struct EntityIter<'a, E, I, V> {
    _v: PhantomData<V>,
    entity_by: &'a E,
    iter: I,
}

impl<'a, E, I, V> Iterator for EntityIter<'a, E, I, V>
where
    E: EntityBy<I::Item, V>,
    I: Iterator,
    V: 'a,
{
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().and_then(|i| self.entity_by.entity_by(i))
    }
}

pub struct ManyIter<'a, MANY>(pub(crate) Iter<'a, MANY>);

impl<'a, MANY> ManyIter<'a, MANY> {
    pub fn is_empty(mut self) -> bool {
        self.0.next().is_none()
    }
}

impl<'a, MANY> Clone for ManyIter<'a, MANY> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<'a, MANY> Iterator for ManyIter<'a, MANY>
where
    MANY: Clone,
{
    type Item = MANY;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().cloned()
    }
}
