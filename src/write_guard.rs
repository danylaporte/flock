use crate::{AsMutOpt, SetTag};
use tokio::sync::RwLockWriteGuard;
use std::ops::{Deref, DerefMut};
use version_tag::VersionTag;

pub struct WriteGuard<T: SetTag + 'static> {
    cancel_tag: bool,
    guard: RwLockWriteGuard<'static, Option<T>>,
    new_tag: VersionTag,
}

impl<T: SetTag> WriteGuard<T> {
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

impl<T: SetTag> AsMut<T> for WriteGuard<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

impl<T: SetTag> AsMutOpt<T> for WriteGuard<T> {
    #[inline]
    fn as_mut_opt(&mut self) -> Option<&mut T> {
        self.guard.as_mut()
    }
}

impl<T: SetTag> AsRef<Option<T>> for WriteGuard<T> {
    #[inline]
    fn as_ref(&self) -> &Option<T> {
        self.guard.deref()
    }
}

impl<T: SetTag> AsRef<T> for WriteGuard<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<T: SetTag> Deref for WriteGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.guard.deref().as_ref().expect("WriteGuard")
    }
}

impl<T: SetTag> DerefMut for WriteGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.guard.deref_mut().as_mut().expect("WriteGuard")
    }
}

impl<T: SetTag + 'static> Drop for WriteGuard<T> {
    fn drop(&mut self) {
        if !self.cancel_tag {
            if let Some(v) = &mut *self.guard {
                v.set_tag(self.new_tag);
            }
        }
    }
}
