use crate::Lock;

pub trait AsLock: Sized + 'static {
    fn as_lock() -> &'static Lock<Self>;
}
