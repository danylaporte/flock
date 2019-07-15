use crate::VersionTag;

pub trait SetTag {
    fn set_tag(&mut self, tag: VersionTag);
}
