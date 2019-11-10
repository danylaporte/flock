use super::db::{ChangeState, IndexMap};

pub struct TableRef<T> {}

pub struct IndexTable<KEY, ENTITY> {
    index_map: IndexMap<KEY, ENTITY>,
}

pub struct ReadGuard<T>;
pub struct WriteGuard<T>;
