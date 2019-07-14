use crate::{map_error, LockValue};
use failure::Error;
use futures::{try_ready, Async, Future, Poll};
use futures_locks::{RwLockReadFut, RwLockReadGuard};
use std::ops::Deref;

pub struct ReadOptGuard<T>(pub(crate) RwLockReadGuard<LockValue<T>>);

impl<T> ReadOptGuard<T> {
    pub fn tag(&self) -> version_tag::VersionTag {
        self.0.deref().1
    }
}

impl<T> Deref for ReadOptGuard<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0.deref().0
    }
}

pub struct ReadOptFut<T>(pub(crate) RwLockReadFut<LockValue<T>>);

impl<T> ReadOptFut<T> {
    pub(crate) fn load(f: RwLockReadFut<LockValue<T>>) -> Self {
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
