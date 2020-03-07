use std::ops::Deref;
use tokio::sync::RwLockReadGuard;

pub struct ReadOptGuard<T: 'static>(pub(crate) RwLockReadGuard<'static, Option<T>>);

impl<T> AsRef<Option<T>> for ReadOptGuard<T> {
    #[inline]
    fn as_ref(&self) -> &Option<T> {
        self.0.deref()
    }
}

impl<T> Deref for ReadOptGuard<T> {
    type Target = Option<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
