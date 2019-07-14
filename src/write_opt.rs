use crate::{map_error, LockValue};
use failure::Error;
use futures::{try_ready, Async, Future, Poll};
use futures_locks::{RwLockWriteFut, RwLockWriteGuard};
use std::ops::Deref;

pub struct WriteOptGuard<T>(pub(crate) RwLockWriteGuard<LockValue<T>>);

impl<T> WriteOptGuard<T> {
    pub fn tag(&self) -> version_tag::VersionTag {
        self.0.deref().1
    }
}

impl<T> Deref for WriteOptGuard<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0.deref().0
    }
}

pub struct WriteOptFut<T>(pub(crate) RwLockWriteFut<LockValue<T>>);

impl<T> WriteOptFut<T> {
    pub(crate) fn load(f: RwLockWriteFut<LockValue<T>>) -> Self {
        Self(f)
    }
}

impl<T> Future for WriteOptFut<T> {
    type Item = WriteOptGuard<T>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let guard = try_ready!(self.0.poll().map_err(map_error));
        Ok(Async::Ready(WriteOptGuard(guard)))
    }
}
