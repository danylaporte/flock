pub trait AsRefTag<T>: AsRef<T> {
    fn tag(&self) -> version_tag::VersionTag;
}
