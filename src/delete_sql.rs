use crate::Result;

pub trait DeleteSql: Sized {
    fn delete_sql<'a>(
        &'a self,
        trans: mssql_client::Transaction,
    ) -> futures03::future::LocalBoxFuture<'a, Result<mssql_client::Transaction>>;
}
