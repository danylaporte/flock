use std::ops::Deref;
use version_tag::VersionTag;

pub struct Tag<T> {
    pub(crate) value: T,
    pub(crate) tag: VersionTag,
}

impl<T> Deref for Tag<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> Tag<T> {
    pub fn tag(&self) -> VersionTag {
        self.tag
    }
}
