use crate::Lock;

/// This is automatically implemented on tables.
///
/// Invoking as_lock returns a locking strategie associated with the table.
pub trait AsLock: Sized + 'static {
    fn as_lock() -> &'static Lock<Self>;
}
