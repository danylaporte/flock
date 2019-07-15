use failure::Error;
use futures::future::Future;
use mssql_client::Connection;

pub trait LoadFromConn: Sized {
    type Future: Future<Item = (Connection, Self), Error = Error>;
    fn load_from_conn(conn: Connection) -> Self::Future;
}
