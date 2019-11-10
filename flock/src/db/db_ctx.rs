use futures_locks::{RwLockReadGuard, RwLockWriteGuard};
use std::marker::PhantomData;

// use crate::db::{Table, TableRef};

// pub struct Ctx<LOCKS> {
//     locks: LOCKS,
// }

pub struct DbCtx<L> {
    locks: L,
}

type Accounts = IndexTable<AccountId, Account>;

impl<L> DbCtx<L> {
    pub fn accounts(&self) -> TableRef<Self, Accounts> {
        TableRef(self)
    }

    pub fn accounts_mut(&mut self) -> TableMut<Self, Accounts> {
        TableMut(self)
    }
}

pub struct Table<CTX, TABLE> {
    ctx: CTX,
    _table: PhantomData<TABLE>,
}

impl<CTX, KEY, ENTITY> Table<CTX, IndexTable<KEY, ENTITY>>
where
    CTX: AsRef<IndexTable<KEY, ENTITY>>,
{
    pub fn get(&self, key: KEY) -> Option<EntityRef<CTX, KEY, ENTITY>> {
        self.ctx.as_ref().get(key)
    }
}

impl<CTX, KEY, ENTITY> TableMut<CTX, IndexTable<KEY, ENTITY>>
where
    CTX: AsMut<IndexTable<KEY, ENTITY>>,
{
    pub fn get_mut(&mut self, key: KEY) -> EntityMut<KEY, ENTITY> {}
}

pub struct EntityRef<CTX, KEY, ENTITY> {
    key: KEY,
    entity: ENTITY,
}

pub struct EntityMut<CTX, KEY, ENTITY> {
    key: KEY,
    entity: ENTITY,
}

pub struct ReadGuard<T>;

impl<T> Deref for ReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// An upgradable read guard allow multiple read and when ready to commit everything, lock for write and do the job (if necessary).
/// No other write or upgradable write can be taken until those lock have been dropped.
pub struct UpgradableIndexTableGuard<KEY, ENTITY> {
    change_tracker: HashMap<KEY, ChangeState<Entity>>,
    index_map: RwLockReadGuard<IndexMap<KEY, ENTITY>>,
}

pub struct WriteGuard<T>;

pub trait IndexTable<KEY, VALUE> {
    fn is_empty(&self) -> bool;
    fn get(&self, key: KEY) -> Option<ENTITY>;
    fn len(&self) -> usize;
}

impl<KEY, VALUE> IndexTable for UpgradableIndexTableGuard<KEY, VALUE>
where
    KEY: Copy,
{
    pub fn is_empty(&self) -> bool {
        let mut len = 0usize;

        for (key, change) in &self.change_tracker {
            match change {
                ChangeState::Inserted(_) | ChangeState::Updated(_) => return true,
                ChangeState::Removed => len += 1,
            }
        }

        // if the index_map has more values then the change_tracker, necessarily the table is not empty
        self.index_map.len() > self.change_tracker.len()

        // if the change tracker has something inserted or updated, necessarily the table is not empty
        || self.change_tracker.values().any(|v| v.is_insert_or_update())

        // if some keys in the index_map are not in the change_tracker, necessarily the table contains those row.
        || self.index_map.keys().any(|k| !self.change_tracker.contains_key(k))
    }

    pub fn len(&self) -> usize {
        let len = self.index_map.len();

        self.change_tracker
            .iter()
            .fold(len, |len, (key, change)| match change {
                ChangeState::Updated(_) if self.index_map.contains_key(*key) => len,
                ChangeState::Updated(_) => len + 1,
                ChangeState::Removed => len - 1,
            })
    }
}
