use crate::{
    AsLock, ConnOrFactory, LoadFromConn, ReadFut, ReadGuard, ReadOptFut, ReadOptGuard, SetTag,
    WriteFut, WriteGuard, WriteOptFut, WriteOptGuard,
};
use failure::Error;
use futures::{Async, Future};
use std::mem::replace;

pub enum LockStates<F, G> {
    Future(F),
    Guard(G),
    None,
}

impl<T> LockStates<ReadFut<T>, ReadGuard<T>>
where
    T: AsLock + LoadFromConn,
{
    pub fn poll(&mut self, conn: &mut Conn) -> Result<bool, Error> {
        if let LockStates::None = self {
            *self = LockStates::Future(T::as_lock().read(conn.take().expect("ConnOrFactory")))
        }

        let state = match self {
            LockStates::Future(f) => match f.poll() {
                Ok(Async::Ready((c, g))) => {
                    *conn = Some(c);
                    LockStates::Guard(g)
                }
                Ok(Async::NotReady) => return Ok(false),
                Err(e) => return Err(e),
            },
            LockStates::Guard(_) => return Ok(true),
            LockStates::None => unreachable!("Cannot poll twice."),
        };

        *self = state;
        Ok(true)
    }
}

impl<T> LockStates<ReadOptFut<T>, ReadOptGuard<T>>
where
    T: AsLock,
{
    pub fn poll(&mut self, _: &mut Conn) -> Result<bool, Error> {
        if let LockStates::None = self {
            *self = LockStates::Future(T::as_lock().read_opt())
        }

        let state = match self {
            LockStates::Future(f) => match f.poll() {
                Ok(Async::Ready(g)) => LockStates::Guard(g),
                Ok(Async::NotReady) => return Ok(false),
                Err(e) => return Err(e),
            },
            LockStates::Guard(_) => return Ok(true),
            LockStates::None => unreachable!("Cannot poll twice."),
        };

        *self = state;
        Ok(true)
    }
}

impl<T> LockStates<WriteFut<T>, WriteGuard<T>>
where
    T: AsLock + LoadFromConn + SetTag,
{
    pub fn poll(&mut self, conn: &mut Conn) -> Result<bool, Error> {
        if let LockStates::None = self {
            *self = LockStates::Future(T::as_lock().write(conn.take().expect("ConnOrFactory")))
        }

        let state = match self {
            LockStates::Future(f) => match f.poll() {
                Ok(Async::Ready((c, g))) => {
                    *conn = Some(c);
                    LockStates::Guard(g)
                }
                Ok(Async::NotReady) => return Ok(false),
                Err(e) => return Err(e),
            },
            LockStates::Guard(_) => return Ok(true),
            LockStates::None => unreachable!("Cannot poll twice."),
        };

        *self = state;
        Ok(true)
    }
}

impl<T> LockStates<WriteOptFut<T>, WriteOptGuard<T>>
where
    T: AsLock + SetTag,
{
    pub fn poll(&mut self, _: &mut Conn) -> Result<bool, Error> {
        if let LockStates::None = self {
            *self = LockStates::Future(T::as_lock().write_opt())
        }

        let state = match self {
            LockStates::Future(f) => match f.poll() {
                Ok(Async::Ready(g)) => LockStates::Guard(g),
                Ok(Async::NotReady) => return Ok(false),
                Err(e) => return Err(e),
            },
            LockStates::Guard(_) => return Ok(true),
            LockStates::None => unreachable!("Cannot poll twice."),
        };

        *self = state;
        Ok(true)
    }
}

type Conn = Option<ConnOrFactory>;

impl<F, G> LockStates<F, G> {
    pub fn take(&mut self) -> G {
        match replace(self, LockStates::None) {
            LockStates::Future(_) => unreachable!("Lock in future state"),
            LockStates::Guard(g) => g,
            LockStates::None => panic!("Already taken"),
        }
    }
}
