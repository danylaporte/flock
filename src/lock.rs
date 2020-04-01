use crate::{
    ConnOrFactory, LoadFromSql, ReadGuard, ReadOptGuard, SetTag, WriteGuard, WriteOptGuard,
};
use failure::Error;
use once_cell::sync::OnceCell;
use tokio::sync::RwLock;

pub struct Lock<T>(OnceCell<RwLock<Option<T>>>);

impl<T> Lock<T> {
    pub const fn new() -> Self {
        Self(OnceCell::new())
    }

    fn get(&'static self) -> &'static RwLock<Option<T>> {
        self.0.get_or_init(|| RwLock::new(None))
    }

    pub async fn read(
        &'static self,
        mut conn: ConnOrFactory,
    ) -> Result<(ConnOrFactory, ReadGuard<T>), Error>
    where
        T: LoadFromSql,
    {
        let lockable = self.get();
        loop {
            // check if the lock has already something in it
            {
                let lock = lockable.read().await;
                if lock.is_some() {
                    return Ok((conn, ReadGuard(lock)));
                }
            }

            // if empty, fill the lock
            {
                let mut lock = lockable.write().await;

                if lock.is_none() {
                    let (c, value) = T::load_from_sql(conn).await?;
                    *lock = Some(value);
                    conn = c;
                }
            }
        }
    }

    pub async fn read_opt(&'static self) -> ReadOptGuard<T> {
        ReadOptGuard(self.get().read().await)
    }

    pub async fn write(
        &'static self,
        mut conn: ConnOrFactory,
    ) -> Result<(ConnOrFactory, WriteGuard<T>), Error>
    where
        T: LoadFromSql + SetTag,
    {
        let mut lock = self.get().write().await;

        if lock.is_none() {
            let (c, value) = T::load_from_sql(conn).await?;
            *lock = Some(value);
            conn = c;
        }

        Ok((conn, WriteGuard::new(lock)))
    }

    pub async fn write_opt(&'static self) -> WriteOptGuard<T>
    where
        T: SetTag,
    {
        WriteOptGuard::new(self.get().write().await)
    }
}

#[tokio::test(threaded_scheduler)]
async fn deadlock_test_read() -> Result<(), Error> {
    use futures03::stream::StreamExt;
    use std::time::Duration;

    static LOCK: Lock<MyTestTable> = Lock::new();

    let stream = futures03::stream::FuturesUnordered::new();

    for i in 0..5 {
        stream.push(async move {
            let conn = ConnOrFactory::from_env("DB")?;
            let guard = LOCK.read(conn).await?;
            tokio::time::delay_for(Duration::from_millis(i * 5)).await;
            drop(guard);

            tokio::time::delay_for(Duration::from_millis(i * 2)).await;

            let mut guard = LOCK.write_opt().await;
            tokio::time::delay_for(Duration::from_millis(i * 3)).await;
            *guard = None;
            Result::<(), Error>::Ok(())
        });
    }

    let _ = stream.collect::<Vec<_>>().await;

    Ok(())
}

#[cfg(test)]
struct MyTestTable;

#[cfg(test)]
impl LoadFromSql for MyTestTable {
    fn load_from_sql(
        conn: ConnOrFactory,
    ) -> futures03::future::LocalBoxFuture<'static, Result<(ConnOrFactory, Self), Error>> {
        Box::pin(async {
            tokio::time::delay_for(std::time::Duration::from_millis(20)).await;
            Ok((conn, Self))
        })
    }
}

#[cfg(test)]
impl SetTag for MyTestTable {
    fn set_tag(&mut self, _tag: version_tag::VersionTag) {}
}
