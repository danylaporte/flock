pub trait AsLock: Sized {
    type Lock;
    fn as_lock() -> Self::Lock;
}
