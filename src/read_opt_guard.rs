use async_std::sync::RwLockReadGuard;
use std::ops::Deref;

pub struct ReadOptGuard<T: 'static>(pub(crate) RwLockReadGuard<'static, Option<T>>);

impl<T> AsRef<Option<T>> for ReadOptGuard<T> {
    fn as_ref(&self) -> &Option<T> {
        self.0.deref()
    }
}

impl<T> Deref for ReadOptGuard<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
