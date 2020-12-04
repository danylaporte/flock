use crate::Result;
use futures03::future::LocalBoxFuture;

pub trait DoLock {
    fn do_lock() -> LocalBoxFuture<'static, Result<Self>>
    where
        Self: Sized;
}
