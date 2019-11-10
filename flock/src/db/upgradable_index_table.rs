use super::{
    change_state::ChangeState,
    change_tracker::ChangeTracker,
    index_map::{IndexMap, Iter as IndexMapIter},
};
use failure::{format_err, Error};
use futures::future::{ok, Future};
use futures_locks::{Mutex, RwLockReadGuard};
use mssql_client::{Parameter, Transaction};
use std::{
    collections::hash_map::{Entry, Values, ValuesMut},
    hash::Hash,
    ops::{Deref, DerefMut},
};

pub trait UpdateDiff {
    /// Compute and update sql query if any values has changes, otherwise returns None.
    fn update_diff(&self, old: &Self) -> Option<(String, Vec<Parameter<'static>>)>;
}

pub struct UpgradableIndexTable<K, V> {
    map: RwLockReadGuard<IndexMap<K, V>>,
    tracker: ChangeTracker<K, V>,
    transaction: Mutex<Option<Transaction>>,
}

impl<K, V> UpgradableIndexTable<K, V> {
    pub fn cancel_changes(&mut self) {
        self.tracker.clear();
    }

    pub fn get(&self, key: K) -> Option<&V>
    where
        K: Eq + Hash + Into<usize>,
    {
        match self.tracker.get(&key) {
            Some(ChangeState::Inserted(v)) | Some(ChangeState::Updated(v)) => Some(v),
            Some(ChangeState::Removed) => None,
            None => self.map.get(key),
        }
    }

    pub fn get_mut(&mut self, key: K) -> Option<IndexMut<K, V>>
    where
        K: Copy + Eq + Hash + Into<usize>,
        V: Clone,
    {
        let tracker_value = self.tracker.entry(key);
        let map_value = self.map.get(key);

        if let Entry::Vacant(_) = tracker_value {
            if map_value.is_none() {
                return None;
            }
        }

        Some(IndexMut {
            local_copy: None,
            map_value,
            tracker_value,
            transaction: &self.transaction,
        })
    }

    pub fn has_changes(&self) -> bool {
        !self.tracker.is_empty()
    }

    pub fn insert(&mut self, key: K, value: V)
    where
        K: Copy + Eq + Hash + Into<usize>,
        V: PartialEq,
    {
        match self.map.get(key) {
            Some(row) if row == &value => {
                self.tracker.remove(&key);
            }
            Some(row) => {
                self.tracker.insert(key, ChangeState::Updated(value));
            }
            None => {
                self.tracker.insert(key, ChangeState::Inserted(value));
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() > 0
    }

    pub fn iter(&self) -> Iter<K, V>
    where
        K: Eq + From<usize> + Hash,
    {
        Iter {
            index_map_iter: self.map.iter(),
            tracker: &self.tracker,
            tracker_iter: self.tracker.values(),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len() - self.tracker.removed_len() + self.tracker.inserted_len()
    }

    pub fn remove(&mut self, key: K)
    where
        K: Copy + Eq + Hash + Into<usize>,
    {
        match self.map.get(key) {
            Some(_) => {
                self.tracker.insert(key, ChangeState::Removed);
            }
            None => {
                self.tracker.remove(&key);
            }
        }
    }
}

pub struct IndexMut<'a, K, V> {
    local_copy: Option<V>,
    map_value: Option<&'a V>,
    tracker_value: Entry<'a, K, ChangeState<V>>,
    transaction: &'a Mutex<Option<Transaction>>,
}

impl<'a, K, V> Deref for IndexMut<'a, K, V> {
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.local_copy
            .as_ref()
            .unwrap_or_else(|| get_old_value(&self.tracker_value, self.map_value))
    }
}

impl<'a, K, V> DerefMut for IndexMut<'a, K, V>
where
    V: Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        let tracker_value = &self.tracker_value;
        let map_value = self.map_value;
        self.local_copy
            .get_or_insert_with(|| get_old_value(tracker_value, map_value).clone())
    }
}

impl<'a, K, V> IndexMut<'a, K, V>
where
    V: Clone + UpdateDiff,
{
    pub fn execute(self) -> Box<dyn Future<Item = (), Error = Error> + 'a>
    where
        K: Copy + Eq + Hash + Into<usize>,
        V: PartialEq,
    {
        // changes appears only if there is a local copy
        let new = match self.local_copy {
            Some(v) => v,
            None => return Box::new(ok::<_, Error>(())),
        };

        let map_value = self.map_value;
        let tracker_value = self.tracker_value;
        let old = get_old_value(&tracker_value, map_value);

        let (sql, params) = match new.update_diff(old) {
            Some(u) => u,
            None => return Box::new(ok::<_, Error>(())),
        };

        Box::new(
            self.transaction
                .lock()
                .map_err(|_| format_err!("Lock"))
                .and_then(move |mut lock| {
                    lock.take()
                        .expect("transaction")
                        .execute(sql, params)
                        .map(move |transaction| {
                            *lock = Some(transaction);

                            match tracker_value {
                                Entry::Occupied(mut o) => match o.get_mut() {
                                    ChangeState::Inserted(v) => {
                                        *v = new;
                                    }
                                    ChangeState::Updated(_) => {
                                        if map_value.expect("map_value") == &new {
                                            o.remove();
                                        } else {
                                            *o.get_mut() = ChangeState::Updated(new);
                                        }
                                    }
                                    ChangeState::Removed => unreachable!("Removed"),
                                },
                                Entry::Vacant(v) => {
                                    v.insert(ChangeState::Updated(new));
                                }
                            }
                        })
                }),
        )
    }
}

fn get_old_value<'a, K, V>(
    tracker_value: &'a Entry<'a, K, ChangeState<V>>,
    map_value: Option<&'a V>,
) -> &'a V {
    match tracker_value {
        Entry::Occupied(o) => match o.get() {
            ChangeState::Inserted(v) | ChangeState::Updated(v) => v,
            ChangeState::Removed => unreachable!("change_tracker_removed"),
        },
        Entry::Vacant(_) => match map_value {
            Some(v) => v,
            None => unreachable!("map_value not found"),
        },
    }
}

pub struct Iter<'a, K, V> {
    index_map_iter: IndexMapIter<'a, K, V>,
    tracker: &'a ChangeTracker<K, V>,
    tracker_iter: Values<'a, K, ChangeState<V>>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: Eq + From<usize> + Hash,
{
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((k, value)) = self.index_map_iter.next() {
            return match self.tracker.get(&k) {
                Some(ChangeState::Inserted(v)) => unreachable!("Insert in both table and tracker"),
                Some(ChangeState::Updated(v)) => Some(v),
                Some(ChangeState::Removed) => continue,
                None => Some(value),
            };
        }

        while let Some(v) = self.tracker_iter.next() {
            if let ChangeState::Inserted(v) = v {
                return Some(v);
            }
        }

        None
    }
}

// pub struct IterMut<'a, K, V> {
//     state: State<'a, K, V>,
// }

// enum State<'a, K, V> {
//     MapIter(IndexMapIter<'a, K, V>, &'a mut ChangeTracker<K, V>),
//     TrackerIter(TrackerIter<'a, K, V>),
//     Done,
// }

// impl<'a, K, V> Iterator for IterMut<'a, K, V>
// where
//     K: Eq + From<usize> + Hash,
//     V: Clone,
// {
//     type Item = &'a mut V;

//     fn next(&mut self) -> Option<Self::Item> {
//         match &mut self.state {
//             State::MapIter(iter, tracker, current) => {
//                 while let Some((k, value)) = self.iter.next() {
//                     return match self.tracker.get(&k) {
//                         Some(ChangeState::Inserted(v)) => unreachable!("Insert in both table and tracker"),
//                         Some(ChangeState::Updated(v)) => Some(v),
//                         Some(ChangeState::Removed) => continue,
//                         None => {
//                             self.value = Some(value.clone());
//                             return self.value.as_mut();
//                         }
//                     };
//                 }
//             }
//         }

//         while let Some((k, value)) = self.index_map_iter.next() {
//             return match self.tracker.get(&k) {
//                 Some(ChangeState::Inserted(v)) => unreachable!("Insert in both table and tracker"),
//                 Some(ChangeState::Updated(v)) => Some(v),
//                 Some(ChangeState::Removed) => continue,
//                 None => {
//                     self.value = Some(value.clone());
//                     return self.value.as_mut();
//                 }
//             };
//         }

//         while let Some((k, v)) = self.tracker_iter.next() {
//             if let ChangeState::Inserted(v) = v {
//                 return Some(v);
//             }
//         }

//         None
//     }
// }
