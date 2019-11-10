use super::change_state::ChangeState;
use std::{
    collections::hash_map::{HashMap, Values},
    hash::Hash,
    iter::IntoIterator,
};
use version_tag::VersionTag;

pub struct HashTable<KEY, ENTITY> {
    map: HashMap<KEY, ENTITY>,
    tag: VersionTag,
}

impl<KEY, ENTITY> HashTable<KEY, ENTITY> {
    pub fn get(&self, key: KEY) -> Option<&ENTITY>
    where
        KEY: Eq + Hash,
    {
        self.map.get(&key)
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn iter(&self) -> Values<KEY, ENTITY> {
        self.map.values()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn tag(&self) -> VersionTag {
        self.tag
    }
}

impl<'a, KEY, ENTITY> IntoIterator for &'a HashTable<KEY, ENTITY> {
    type Item = &'a ENTITY;
    type IntoIter = Values<'a, KEY, ENTITY>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
