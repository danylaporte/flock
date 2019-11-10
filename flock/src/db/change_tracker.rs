use super::change_state::ChangeState;
use std::{
    collections::hash_map::{Entry, HashMap, Values, ValuesMut},
    hash::Hash,
};

pub(crate) struct ChangeTracker<K, V> {
    inserted_len: usize,
    map: HashMap<K, ChangeState<V>>,
    removed_len: usize,
    updated_len: usize,
}

impl<K, V> ChangeTracker<K, V> {
    pub fn clear(&mut self) {
        self.map.clear();
        self.inserted_len = 0;
        self.removed_len = 0;
        self.updated_len = 0;
    }

    pub fn entry(&mut self, key: K) -> Entry<K, ChangeState<V>>
    where
        K: Eq + Hash,
    {
        self.map.entry(key)
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn get(&self, key: &K) -> Option<&ChangeState<V>>
    where
        K: Eq + Hash,
    {
        self.map.get(key)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut ChangeState<V>>
    where
        K: Eq + Hash,
    {
        self.map.get_mut(key)
    }

    pub fn insert(&mut self, key: K, value: ChangeState<V>) -> Option<ChangeState<V>>
    where
        K: Eq + Hash,
    {
        self.update_counter(&value, add_one);
        let old = self.map.insert(key, value)?;
        self.update_counter(&old, sub_one);

        Some(old)
    }

    pub fn inserted_len(&self) -> usize {
        self.inserted_len
    }

    pub fn remove(&mut self, key: &K) -> Option<ChangeState<V>>
    where
        K: Eq + Hash,
    {
        let old = self.map.remove(key)?;
        self.update_counter(&old, sub_one);
        Some(old)
    }

    pub fn removed_len(&self) -> usize {
        self.removed_len
    }

    pub fn updated_len(&self) -> usize {
        self.updated_len
    }

    fn update_counter(&mut self, v: &ChangeState<V>, op: impl FnOnce(&mut usize)) {
        match v {
            ChangeState::Inserted(_) => op(&mut self.inserted_len),
            ChangeState::Removed => op(&mut self.removed_len),
            ChangeState::Updated(_) => op(&mut self.updated_len),
        }
    }

    pub fn values(&self) -> Values<K, ChangeState<V>> {
        self.map.values()
    }

    pub fn values_mut(&mut self) -> ValuesMut<K, ChangeState<V>> {
        self.map.values_mut()
    }
}

fn add_one(v: &mut usize) {
    *v += 1;
}

fn sub_one(v: &mut usize) {
    *v -= 1;
}
