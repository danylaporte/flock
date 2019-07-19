use crate::{map_error, ConnOrFactory, LoadFromConn};
use failure::Error;
use futures::{try_ready, Async, Future, Poll};
use futures_locks::{RwLock, RwLockReadFut, RwLockReadGuard, RwLockWriteFut, RwLockWriteGuard};
use mssql_client::Connection;
use std::ops::Deref;

pub struct ReadGuard<T>(pub(crate) RwLockReadGuard<Option<T>>);

impl<T> AsRef<T> for ReadGuard<T> {
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<T> AsRef<Option<T>> for ReadGuard<T> {
    fn as_ref(&self) -> &Option<T> {
        self.0.deref()
    }
}

impl<T> Deref for ReadGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.0.deref().as_ref().expect("ReadGuard")
    }
}

pub struct ReadFut<T: LoadFromConn> {
    conn_or_factory: Option<ConnOrFactory>,
    lock: RwLock<Option<T>>,
    state: State<T>,
}

impl<T: LoadFromConn> ReadFut<T> {
    pub(crate) fn load(conn_or_factory: ConnOrFactory, lock: RwLock<Option<T>>) -> Self {
        Self {
            conn_or_factory: Some(conn_or_factory),
            state: State::Read(lock.read()),
            lock,
        }
    }
}

impl<T: LoadFromConn> Future for ReadFut<T> {
    type Item = (ConnOrFactory, ReadGuard<T>);
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let state = match &mut self.state {
                State::Connect(guard, f) => {
                    let conn = try_ready!(f.poll());
                    let guard = guard.take().expect("Guard");

                    State::Load(guard, T::load_from_conn(conn))
                }

                State::Load(guard, f) => {
                    let (conn, value) = try_ready!(f.poll());

                    self.conn_or_factory = Some(ConnOrFactory::Connection(conn));
                    **guard = Some(value);

                    State::Read(self.lock.read())
                }

                State::Read(f) => {
                    let guard = try_ready!(f.poll().map_err(map_error));

                    if guard.is_some() {
                        let conn_or_factory = self.conn_or_factory.take().expect("ConnOrFactory");
                        return Ok(Async::Ready((conn_or_factory, ReadGuard(guard))));
                    }

                    State::Write(self.lock.write())
                }

                State::Write(f) => {
                    let guard = try_ready!(f.poll().map_err(map_error));

                    if guard.is_some() {
                        State::Read(self.lock.read())
                    } else {
                        match self.conn_or_factory.take().expect("ConnOrFactory") {
                            ConnOrFactory::Connection(c) => {
                                State::Load(guard, T::load_from_conn(c))
                            }
                            ConnOrFactory::Factory(f) => {
                                State::Connect(Some(guard), Box::new(f.create_connection()))
                            }
                        }
                    }
                }
            };

            self.state = state;
        }
    }
}

enum State<T: LoadFromConn> {
    Connect(
        Option<RwLockWriteGuard<Option<T>>>,
        Box<dyn Future<Item = Connection, Error = Error>>,
    ),
    Load(RwLockWriteGuard<Option<T>>, T::Future),
    Read(RwLockReadFut<Option<T>>),
    Write(RwLockWriteFut<Option<T>>),
}
