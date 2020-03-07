use crate::{
    ConnOrFactory, LoadFromSql, ReadGuard, ReadOptGuard, SetTag, WriteGuard, WriteOptGuard,
};
use tokio::sync::RwLock;
use failure::Error;
use once_cell::sync::OnceCell;

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
