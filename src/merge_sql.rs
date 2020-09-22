pub trait MergeSql: Sized {
    fn merge_sql<'a>(
        &'a self,
        trans: mssql_client::Transaction,
    ) -> futures03::future::LocalBoxFuture<'a, Result<mssql_client::Transaction, failure::Error>>;
}
