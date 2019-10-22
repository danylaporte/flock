use version_tag::VersionTag;

pub trait SetTag {
    fn set_tag(&mut self, tag: VersionTag);
}
