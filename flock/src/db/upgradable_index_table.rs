use super::{
    change_state::ChangeState,
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
    vec::IntoIter,
};

pub trait InsertSql {
    fn insert_sql(&self) -> (String, Vec<Parameter<'static>>);
}

pub trait UpdateSqlDiff {
    /// Compute and update sql query if any values has changes, otherwise returns None.
    fn update_sql_diff(&self, old: &Self) -> Option<(String, Vec<Parameter<'static>>)>;
}

pub struct UpgradableIndexTable<K, V> {
    changes: IndexMap<K, ChangeState<V>>,
    map: RwLockReadGuard<IndexMap<K, V>>,
    transaction: Mutex<Option<Transaction>>,
}

impl<K, V> UpgradableIndexTable<K, V> {
    pub fn get(&self, key: K) -> Option<&V>
    where
        K: Copy + Into<usize>,
    {
        match self.changes.get(key) {
            Some(ChangeState::Inserted(v)) | Some(ChangeState::Updated(v)) => Some(*v),
            Some(ChangeState::Removed) | None => None,
            Some(ChangeState::Unchanged) => self.map.get(key),
        }
    }

    pub fn get_mut(&mut self, key: K) -> Option<IndexMut<K, V>>
    where
        K: Copy + Eq + Hash + Into<usize>,
        V: Clone,
    {
        match self.tracker.get(&key) {
            Some(ChangeState::Inserted(_)) | Some(ChangeState::Updated(_)) => {}
            Some(ChangeState::Removed) => return None,
            None => match self.map.get(key) {
                Some(_) => {}
                None => return None,
            },
        }

        Some(IndexMut {
            key,
            local_copy: None,
            map: &self.map,
            tracker: &mut self.tracker,
            transaction: &self.transaction,
        })
    }

    pub fn has_changes(&self) -> bool {
        self.tracker.iter().any(|(_, v)| match v {
            ChangeState::Removed | ChangeState::Updated(_) => true,
            ChangeState::Unchanged => false,
        })
    }

    pub fn insert<'a>(
        &'a mut self,
        key: K,
        value: V,
    ) -> Box<dyn Future<Item = (), Error = Error> + 'a>
    where
        K: Copy + Into<usize>,
        V: InsertSql + UpdateSqlDiff,
    {
        let (sql, params) = match self.changes.get(key) {
            Some(ChangeState::Removed) | None => value.insert_sql(),
            Some(ChangeState::Unchanged) => {
                match value.update_sql_diff(self.map.get(key).expect("map")) {
                    Some(v) => v,
                    None => return,
                }
            }
            Some(ChangeState::Updated(old)) => match value.update_sql_diff(old) {
                Some(v) => v,
                None => return,
            },
        };

        self.transaction
            .lock()
            .map_err(lock_error)
            .and_then(|mut lock| {
                lock.take()
                    .expect("transaction")
                    .exec(sql, params)
                    .map(move |transaction| {
                        *lock = Some(transaction);

                        self.changes.insert(key, match self.map.get(key) {
                            Some(v) if v == &value => ChangeState::Unchanged,
                            _ => ChangeState::Updated(value),
                        });

                        ()
                    })
            })
    }

    pub fn is_empty(&self) -> bool {
        self.len() > 0
    }

    // pub fn iter(&self) -> Iter<K, V>
    // where
    //     K: Eq + From<usize> + Hash,
    // {
    //     Iter {
    //         index_map_iter: self.map.iter(),
    //         tracker: &self.tracker,
    //         tracker_iter: self.tracker.values(),
    //     }
    // }

    // pub fn iter_mut(&mut self) -> IterMut<K, V>
    // where
    //     K: Copy + Eq + From<usize> + Hash,
    // {
    //     let it = self
    //         .map
    //         .iter()
    //         .map(|(k, _)| k)
    //         .filter(|k| match self.tracker.get(k) {
    //             Some(ChangeState::Updated(_)) | None => true,
    //             _ => false,
    //         })
    //         .chain(
    //             self.tracker
    //                 .iter()
    //                 .filter(|(k, v)| match v {
    //                     ChangeState::Inserted(_) => true,
    //                     _ => false,
    //                 })
    //                 .map(|(k, _)| *k),
    //         )
    //         .collect::<Vec<_>>()
    //         .into_iter();

    //     IterMut {
    //         it,
    //         map: &self.map,
    //         tracker: &mut self.tracker,
    //         transaction: &self.transaction,
    //     }
    // }

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
    key: K,
    local_copy: Option<V>,
    map: &'a IndexMap<K, V>,
    tracker: &'a mut ChangeTracker<K, V>,
    transaction: &'a Mutex<Option<Transaction>>,
}

impl<'a, K, V> Deref for IndexMut<'a, K, V>
where
    K: Copy + Eq + Hash + Into<usize>,
{
    type Target = V;

    fn deref(&self) -> &Self::Target {
        self.local_copy
            .as_ref()
            .unwrap_or_else(|| get_old_value(self.key, self.map, self.tracker))
    }
}

impl<'a, K, V> DerefMut for IndexMut<'a, K, V>
where
    K: Copy + Eq + Hash + Into<usize>,
    V: Clone,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        let key = self.key;
        let map = self.map;
        let tracker = &self.tracker;
        self.local_copy
            .get_or_insert_with(|| get_old_value(key, map, tracker).clone())
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

        let key = self.key;
        let map = self.map;
        let tracker = self.tracker;
        let old = get_old_value(key, map, tracker);

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

                            match tracker.entry(key) {
                                Entry::Occupied(mut o) => match o.get_mut() {
                                    ChangeState::Inserted(v) => {
                                        *v = new;
                                    }
                                    ChangeState::Updated(_) => {
                                        if map.get(key).expect("map_value") == &new {
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

fn lock_error<T>(_: T) -> Error {
    unreachable!()
}

fn get_old_value<'a, K, V>(
    key: K,
    map: &'a IndexMap<K, V>,
    tracker: &'a ChangeTracker<K, V>,
) -> &'a V
where
    K: Eq + Hash + Into<usize>,
{
    match tracker.get(&key) {
        Some(ChangeState::Inserted(v)) | Some(ChangeState::Updated(v)) => v,
        Some(ChangeState::Removed) => unreachable!("tracker removed"),
        None => match map.get(key) {
            Some(v) => v,
            None => unreachable!("map not found"),
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

pub struct IterMut<'a, K, V> {
    it: IntoIter<K>,
    map: &'a IndexMap<K, V>,
    tracker: &'a mut ChangeTracker<K, V>,
    transaction: &'a Mutex<Option<Transaction>>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V>
where
    K: Eq + Hash + Into<usize> + 'a,
    V: 'a,
{
    type Item = IndexMut<'a, K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        let key = self.it.next()?;

        Some(IndexMut {
            key,
            local_copy: None,
            map: self.map,
            tracker: self.tracker as _,
            transaction: self.transaction,
        })
    }
}
