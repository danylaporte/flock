use async_std::sync::RwLockReadGuard;
use std::ops::Deref;

pub struct ReadGuard<T: 'static>(pub(crate) RwLockReadGuard<'static, Option<T>>);

impl<T> AsRef<T> for ReadGuard<T> {
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<T> AsRef<Option<T>> for ReadGuard<T> {
    fn as_ref(&self) -> &Option<T> {
        self.0.deref()
    }
}

impl<T> Deref for ReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0.deref().as_ref().expect("ReadGuard")
    }
}
