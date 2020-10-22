use crate::{ConnOrFactory, Result};
use futures03::future::LocalBoxFuture;

pub trait LoadFromSql: Sized {
    fn load_from_sql(conn: ConnOrFactory)
        -> LocalBoxFuture<'static, Result<(ConnOrFactory, Self)>>;
}
