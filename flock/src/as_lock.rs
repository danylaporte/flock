use crate::Lock;

/// This is automatically implemented on tables.
///
/// Invoking as_lock returns a locking strategy associated with the table.
pub trait AsLock: Sized + 'static {
    fn as_lock() -> &'static Lock<Self>;
}
