use crate::{ConnOrFactory, LoadFromConn, ReadFut, ReadOptFut, SetTag, WriteFut, WriteOptFut};
use futures_locks::RwLock;
use once_cell::sync::OnceCell;

pub struct Lock<T>(OnceCell<RwLock<Option<T>>>);

impl<T> Lock<T> {
    pub const fn new() -> Self {
        Self(OnceCell::new())
    }

    fn get(&self) -> &RwLock<Option<T>> {
        self.0.get_or_init(|| RwLock::new(None))
    }

    pub fn read(&self, conn: ConnOrFactory) -> ReadFut<T>
    where
        T: LoadFromConn,
    {
        ReadFut::load(conn, self.get().clone())
    }

    pub fn read_opt(&self) -> ReadOptFut<T> {
        ReadOptFut::load(self.get().read())
    }

    pub fn write(&self, conn: ConnOrFactory) -> WriteFut<T>
    where
        T: LoadFromConn + SetTag,
    {
        WriteFut::load(conn, self.get().write())
    }

    pub fn write_opt(&self) -> WriteOptFut<T>
    where
        T: SetTag,
    {
        WriteOptFut::load(self.get().write())
    }
}
