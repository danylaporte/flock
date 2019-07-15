use crate::{map_error, SetTag, VersionTag};
use failure::Error;
use futures::{try_ready, Async, Future, Poll};
use futures_locks::{RwLockWriteFut, RwLockWriteGuard};
use std::ops::{Deref, DerefMut};

pub struct WriteOptGuard<T: SetTag> {
    cancel_tag: bool,
    guard: RwLockWriteGuard<Option<T>>,
    new_tag: VersionTag,
}

impl<T: SetTag> WriteOptGuard<T> {
    /// Prevent the new_tag value to be placed on the locked value.
    ///
    /// Use this method when there is no changes occurred.
    pub fn cancel_tag(&mut self) {
        self.cancel_tag = true;
    }

    /// The new tag that will be put on the lock value on drop.
    ///
    /// It can be prevent by using `cancel_tag`.
    pub fn new_tag(&self) -> VersionTag {
        self.new_tag
    }
}

impl<T: SetTag> Deref for WriteOptGuard<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<T: SetTag> DerefMut for WriteOptGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

impl<T: SetTag> Drop for WriteOptGuard<T> {
    fn drop(&mut self) {
        if !self.cancel_tag {
            if let Some(g) = &mut *self.guard {
                g.set_tag(self.new_tag);
            }
        }
    }
}

pub struct WriteOptFut<T>(pub(crate) RwLockWriteFut<Option<T>>);

impl<T: SetTag> WriteOptFut<T> {
    pub(crate) fn load(f: RwLockWriteFut<Option<T>>) -> Self {
        Self(f)
    }
}

impl<T: SetTag> Future for WriteOptFut<T> {
    type Item = WriteOptGuard<T>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let guard = try_ready!(self.0.poll().map_err(map_error));
        let guard = WriteOptGuard {
            cancel_tag: false,
            guard,
            new_tag: VersionTag::new(),
        };

        Ok(Async::Ready(guard))
    }
}
