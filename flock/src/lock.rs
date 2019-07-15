use crate::{ConnOrFactory, LoadFromConn, ReadFut, ReadOptFut, SetTag, WriteFut, WriteOptFut};
use futures_locks::RwLock;

pub struct Lock<T>(RwLock<Option<T>>);

impl<T> Lock<T> {
    pub fn new(value: Option<T>) -> Self {
        Self(RwLock::new(value))
    }

    pub fn read<C>(&self, conn: C) -> ReadFut<T>
    where
        C: Into<ConnOrFactory>,
        T: LoadFromConn,
    {
        ReadFut::load(conn.into(), self.0.clone())
    }

    pub fn read_opt(&self) -> ReadOptFut<T> {
        ReadOptFut::load(self.0.read())
    }

    pub fn write<C>(&self, conn: C) -> WriteFut<T>
    where
        C: Into<ConnOrFactory>,
        T: LoadFromConn + SetTag,
    {
        WriteFut::load(conn.into(), self.0.write())
    }

    pub fn write_opt(&self) -> WriteOptFut<T>
    where
        T: SetTag,
    {
        WriteOptFut::load(self.0.write())
    }
}

impl<T> Default for Lock<T> {
    fn default() -> Self {
        Self(RwLock::new(None))
    }
}
