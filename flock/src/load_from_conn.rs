use failure::Error;
use futures::future::Future;
use mssql_client::Connection;

pub type LoadFromConnFut<T> = Box<dyn Future<Item = (Connection, T), Error = Error>>;

pub trait LoadFromConn: Sized {
    fn load_from_conn(conn: Connection) -> LoadFromConnFut<Self>;
}
