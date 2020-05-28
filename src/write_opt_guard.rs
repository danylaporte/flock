use crate::{AsMutOpt, SetTag};
use std::ops::{Deref, DerefMut};
use tokio::sync::RwLockWriteGuard;
use version_tag::VersionTag;

pub struct WriteOptGuard<T: SetTag + 'static> {
    cancel_tag: bool,
    guard: RwLockWriteGuard<'static, Option<T>>,
    new_tag: VersionTag,
}

impl<T: SetTag> AsMut<Option<T>> for WriteOptGuard<T> {
    fn as_mut(&mut self) -> &mut Option<T> {
        self.guard.deref_mut()
    }
}

impl<T: SetTag> AsMutOpt<T> for WriteOptGuard<T> {
    fn as_mut_opt(&mut self) -> Option<&mut T> {
        self.guard.as_mut()
    }
}

impl<T: SetTag> AsRef<Option<T>> for WriteOptGuard<T> {
    fn as_ref(&self) -> &Option<T> {
        self.guard.deref()
    }
}

impl<T: SetTag> WriteOptGuard<T> {
    pub(crate) fn new(guard: RwLockWriteGuard<'static, Option<T>>) -> Self {
        Self {
            cancel_tag: false,
            guard,
            new_tag: VersionTag::new(),
        }
    }
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

impl<T: SetTag + 'static> Drop for WriteOptGuard<T> {
    fn drop(&mut self) {
        if !self.cancel_tag {
            if let Some(g) = &mut *self.guard {
                g.set_tag(self.new_tag);
            }
        }
    }
}
