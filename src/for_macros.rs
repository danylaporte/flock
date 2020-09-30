use std::fmt::Debug;

use crate::{ConnOrFactory, Result, VecOpt};
use futures03::future::LocalBoxFuture;
use mssql_client::{Params, Result as SqlResult, Row};

pub fn load_single_key<'a, F, T, K>(
    conn: ConnOrFactory,
    map: &'a mut VecOpt<T>,
    key: Option<K>,
    sql: &'static str,
    func: F,
) -> LocalBoxFuture<'a, Result<ConnOrFactory>>
where
    K: Clone + Debug + Into<usize> + Params<'static> + 'static,
    F: for<'b> FnMut(&'b mut VecOpt<T>, &Row) -> SqlResult<&'b mut VecOpt<T>> + 'static,
    T: 'static,
{
    map.remove_or_clear(key.clone().map(Into::into));

    Box::pin(async move {
        let conn = conn
            .connect()
            .await?
            .query_fold_imp(sql, key, map, func)
            .await?
            .0;

        Ok(ConnOrFactory::Connection(conn))
    })
}

pub fn load_single_key_translate<'a, F, T, K, TF>(
    conn: ConnOrFactory,
    map: &'a mut VecOpt<T>,
    key: Option<K>,
    sql: &'static str,
    translate_sql: &'static str,
    func: F,
    translate_func: TF,
) -> LocalBoxFuture<'a, Result<ConnOrFactory>>
where
    K: Clone + Debug + Into<usize> + Params<'static> + 'static,
    F: for<'b> FnMut(&'b mut VecOpt<T>, &Row) -> SqlResult<&'b mut VecOpt<T>> + 'static,
    T: 'static,
    TF: for<'b> FnMut(&'b mut VecOpt<T>, &Row) -> SqlResult<&'b mut VecOpt<T>> + 'static,
{
    map.remove_or_clear(key.clone().map(Into::into));

    Box::pin(async move {
        let (conn, map) = conn
            .connect()
            .await?
            .query_fold_imp(sql, key.clone(), map, func)
            .await?;

        let conn = conn
            .query_fold_imp(translate_sql, key, map, translate_func)
            .await?
            .0;

        Ok(ConnOrFactory::Connection(conn))
    })
}
