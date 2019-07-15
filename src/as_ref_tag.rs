use crate::Tag;

pub trait AsRefTag<T>: AsRef<T> {
    fn as_ref_tag(&self) -> Tag<&T>;
}
