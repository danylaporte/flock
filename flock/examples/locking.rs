#![feature(proc_macro_hygiene)]

use flock::{
    failure::Error, futures::Future, mssql_client::Connection, AsLock, LoadFromConn, Lock,
};
use flock_derive::locks;
use tokio::executor::current_thread::block_on_all;

fn main() {
    let fut = locks!(read: [Accounts]).map(|_locks| {});

    block_on_all(fut).unwrap();
}

pub struct Accounts;

impl AsLock for Accounts {
    fn as_lock() -> &'static Lock<Self> {
        unimplemented!()
    }
}

impl LoadFromConn for Accounts {
    fn load_from_conn(
        _conn: Connection,
    ) -> Box<dyn Future<Item = (Connection, Self), Error = Error>> {
        unimplemented!()
    }
}
