use futures_locks::RwLockWriteGuard;
use std::ops::{Deref, DerefMut};

pub struct WGuard<T>(pub(crate) RwLockWriteGuard<Option<T>>);

impl<T> Deref for WGuard<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T> DerefMut for WGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}
