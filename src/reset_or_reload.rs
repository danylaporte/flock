use crate::{ConnOrFactory, Result};
use futures03::future::LocalBoxFuture;

pub trait ResetOrReload<K> {
    fn reset_or_reload(
        fac: ConnOrFactory,
        key: K,
    ) -> LocalBoxFuture<'static, Result<ConnOrFactory>>;
}
