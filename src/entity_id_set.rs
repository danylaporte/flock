use indexmap::IndexSet;
use once_cell::sync::OnceCell;
use parking_lot::{Mutex, MutexGuard};
use uuid::Uuid;

#[doc(hidden)]
pub struct EntityIdSet(OnceCell<Mutex<IndexSet<Uuid>>>);

impl EntityIdSet {
    pub const fn new() -> Self {
        Self(OnceCell::new())
    }

    pub fn capacity(&self) -> usize {
        self.get().len()
    }

    fn get(&self) -> MutexGuard<IndexSet<Uuid>> {
        self.0.get_or_init(|| Mutex::new(IndexSet::new())).lock()
    }

    pub fn get_index(&self, uuid: &Uuid) -> Option<usize> {
        self.get().get_full(uuid).map(|t| t.0)
    }

    pub fn get_or_create_index(&self, uuid: Uuid) -> usize {
        self.get().insert_full(uuid).0.into()
    }

    pub fn get_uuid_unchecked(&self, index: usize) -> Uuid {
        match self.get().get_index(index) {
            Some(id) => *id,
            None => panic!("Uuid"),
        }
    }
}
