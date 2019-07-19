use crate::map_error;
use failure::Error;
use futures::{try_ready, Async, Future, Poll};
use futures_locks::{RwLockReadFut, RwLockReadGuard};
use std::ops::Deref;

pub struct ReadOptGuard<T>(pub(crate) RwLockReadGuard<Option<T>>);

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

pub struct ReadOptFut<T>(pub(crate) RwLockReadFut<Option<T>>);

impl<T> ReadOptFut<T> {
    pub(crate) fn load(f: RwLockReadFut<Option<T>>) -> Self {
        Self(f)
    }
}

impl<T> Future for ReadOptFut<T> {
    type Item = ReadOptGuard<T>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let guard = try_ready!(self.0.poll().map_err(map_error));
        Ok(Async::Ready(ReadOptGuard(guard)))
    }
}
