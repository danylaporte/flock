#![feature(proc_macro_hygiene)]

use flock::{
    failure::Error, futures03::future::LocalBoxFuture, locks, version_tag::VersionTag, AsLock,
    ConnOrFactory, LoadFromSql, Lock, SetTag,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    {
        let _locks = locks!(read: [Accounts]).await?;
    }
    {
        let _locks = locks!(read_opt: [Accounts]).await?;
    }
    {
        let _locks = locks!(write: [Accounts]).await?;
    }
    {
        let _locks = locks!(write_opt: [Accounts]).await?;
    }
    Ok(())
}

pub struct Accounts;

impl AsLock for Accounts {
    fn as_lock() -> &'static Lock<Self> {
        unimplemented!()
    }
}

impl LoadFromSql for Accounts {
    fn load_from_sql(
        _conn: ConnOrFactory,
    ) -> LocalBoxFuture<'static, Result<(ConnOrFactory, Self), Error>> {
        unimplemented!()
    }
}

impl SetTag for Accounts {
    fn set_tag(&mut self, _: VersionTag) {}
}
