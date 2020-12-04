use flock::{
    futures03::future::LocalBoxFuture, locks, version_tag::VersionTag, AsLock, ConnOrFactory,
    LoadFromSql, Lock, Result, SetTag,
};

#[tokio::main]
async fn main() -> Result<()> {
    {
        locks!(L, read: [Accounts]);
        L::lock().await?;
    }
    {
        locks!(L, read_opt: [Accounts]);
        L::lock().await?;
    }
    {
        locks!(L, write: [Accounts]);
        L::lock().await?;
    }
    {
        locks!(L, write_opt: [Accounts]);
        L::lock().await?;
    }
    Ok(())
}

pub struct Accounts;

impl Accounts {
    pub fn tag(&self) -> VersionTag {
        VersionTag::zero()
    }
}

impl AsLock for Accounts {
    fn as_lock() -> &'static Lock<Self> {
        unimplemented!()
    }
}

impl LoadFromSql for Accounts {
    fn load_from_sql(
        _conn: ConnOrFactory,
    ) -> LocalBoxFuture<'static, Result<(ConnOrFactory, Self)>> {
        unimplemented!()
    }
}

impl SetTag for Accounts {
    fn set_tag(&mut self, _: VersionTag) {}
}
