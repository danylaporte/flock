use crate::db::{change_state::ChangeState, IndexMap};

pub enum ChangeState<V> {
    Changed(Box<V>),
    Deleted,
    Pristine,
    Void,
}

pub struct IndexChangeTracker<K, V> {
    len: usize,
    vec: Vec<ChangeState<V>>,
    iter_ready: bool,
}

impl<K, V> IndexChangeTracker<K, V> {
    fn extend(&mut self, map: &IndexMap<K, V>, index: usize) where K: From<usize> + Into<usize> {
        let len = self.vec.len();
    
        if len <= index {
            self.vec.extend((len..=index).into_iter().map(|i| match map.get(i.into()) {
                    Some(_) => ChangeState::Pristine,
                    None => ChangeState::Void,
            }));
        }
    }

    pub fn get_mut<'a>(&mut self, map: &IndexMap<K, V>, key: K) -> Option<ChangeState<V>> where K: Into<usize> {
        let index = key.into();

        match self.vec.get_mut(index) {
            
            Some(ChangeState::Void) => return None,
            Some(ChangeState::Remove) |  => return None,
            Some(ChangeState::Pristine) | Some(ChangeState::)
        }

        if self.vec.get_mut(index) {

        }

        self.extend(map, key);
    }

    pub fn insert(&mut self, key: K, value: ChangeState<V>) {

    }

    pub fn len(&self) -> usize {
        self.len
    }
}
