use crate::{
    ConnOrFactory, LoadFromSql, Lock, ReadGuard, ReadOptGuard, Result, SetTag, WriteGuard,
    WriteOptGuard,
};

/// This is automatically implemented on tables.
///
/// Invoking as_lock returns a locking strategy associated with the table.
pub trait AsLock: Sized + 'static {
    fn as_lock() -> &'static Lock<Self>;
}

pub async fn lock_read<T: AsLock + LoadFromSql>(
    conn: ConnOrFactory,
) -> Result<(ConnOrFactory, ReadGuard<T>)> {
    T::as_lock().read(conn).await
}

pub async fn lock_read_opt<T: AsLock>() -> ReadOptGuard<T> {
    T::as_lock().read_opt().await
}

pub async fn lock_write<T: AsLock + LoadFromSql + SetTag>(
    conn: ConnOrFactory,
) -> Result<(ConnOrFactory, WriteGuard<T>)> {
    T::as_lock().write(conn).await
}

pub async fn lock_write_opt<T: AsLock + SetTag>() -> WriteOptGuard<T> {
    T::as_lock().write_opt().await
}
