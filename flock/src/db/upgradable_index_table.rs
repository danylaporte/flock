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
    borrow::Cow,
    collections::hash_map::{Values, ValuesMut},
    hash::Hash,
    ops::{Deref, DerefMut},
};

pub trait UpdateDiff {
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

    // pub fn get_mut(&mut self, key: K) -> Option<IndexMutGuard<K, V>>
    // where
    //     K: Copy + Eq + Hash + Into<usize>,
    //     V: Clone + PartialEq,
    // {
    //     let value = self.get(key)?.clone();

    //     Some(IndexMutGuard {
    //         key,
    //         table: self,
    //         value: Some(value),
    //     })
    // }

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

pub struct IndexMutGuard<'a, K, V>
where
    V: Clone,
{
    key: K,
    map: &'a IndexMap<K, V>,
    tracker: &'a mut ChangeTracker<K, V>,
    transaction: &'a Mutex<Option<Transaction>>,
    value: Cow<'a, V>,
}

impl<'a, K, V> Deref for IndexMutGuard<'a, K, V>
where
    V: Clone,
{
    type Target = V;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a, K, V> DerefMut for IndexMutGuard<'a, K, V>
where
    V: Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value.to_mut()
    }
}

impl<'a, K, V> IndexMutGuard<'a, K, V>
where
    V: Clone + UpdateDiff,
{
    pub fn execute(self) -> Box<dyn Future<Item = (), Error = Error> + 'a>
    where
        K: Copy + Eq + Hash + Into<usize>,
    {
        // changes appears only on owned value, borrowed value are read only.
        let value = match self.value {
            Cow::Owned(v) => v,
            Cow::Borrowed(_) => return Box::new(ok::<_, Error>(())),
        };

        let key = self.key;
        let tracker = self.tracker;

        // check if the owned value as change or the value is still the same as it was.
        let update_statement = match tracker.get(&key) {
            Some(ChangeState::Inserted(v)) | Some(ChangeState::Updated(v)) => value.update_diff(v),
            Some(ChangeState::Removed) => unreachable!("IndexMutGuard - removed"),
            None => match self.map.get(key) {
                Some(v) => value.update_diff(v),
                None => unreachable!("IndexMutGuard - not found"),
            },
        };

        let (sql, params) = match update_statement {
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

                            if let Some(ChangeState::Inserted(v)) | Some(ChangeState::Updated(v)) =
                                tracker.get_mut(&key)
                            {
                                *v = value;
                                return;
                            }

                            tracker.insert(key, ChangeState::Updated(value));
                        })
                }),
        )
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
