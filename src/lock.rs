use crate::{ConnOrFactory, LoadFromConn, LockValue, ReadFut, ReadOptFut, WriteFut, WriteOptFut};
use futures_locks::RwLock;

pub struct Lock<T> {
    lock: RwLock<LockValue<T>>,
}

impl<T> Lock<T> {
    pub fn read<C>(&self, conn: C) -> ReadFut<T>
    where
        C: Into<ConnOrFactory>,
        T: LoadFromConn,
    {
        ReadFut::load(conn.into(), self.lock.clone())
    }

    pub fn read_opt(&self) -> ReadOptFut<T> {
        ReadOptFut::load(self.lock.read())
    }

    pub fn write<C>(&self, conn: C) -> WriteFut<T>
    where
        C: Into<ConnOrFactory>,
        T: LoadFromConn,
    {
        WriteFut::load(conn.into(), self.lock.write())
    }

    pub fn write_opt(&self) -> WriteOptFut<T> {
        WriteOptFut::load(self.lock.write())
    }
}
