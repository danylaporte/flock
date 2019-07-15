use crate::{map_error, ConnOrFactory, LoadFromConn, SetTag};
use failure::Error;
use futures::{try_ready, Async, Future, Poll};
use futures_locks::{RwLockWriteFut, RwLockWriteGuard};
use mssql_client::Connection;
use std::ops::{Deref, DerefMut};
use version_tag::VersionTag;

pub struct WriteGuard<T: SetTag> {
    cancel_tag: bool,
    guard: RwLockWriteGuard<Option<T>>,
    new_tag: VersionTag,
}

impl<T: SetTag> WriteGuard<T> {
    fn new(guard: RwLockWriteGuard<Option<T>>) -> Self {
        Self {
            cancel_tag: false,
            guard,
            new_tag: VersionTag::new(),
        }
    }

    /// Prevent the new_tag value to be placed on the locked value.
    ///
    /// Use this method when there is no changes occurred.
    pub fn cancel_tag(&mut self) {
        self.cancel_tag = true;
    }

    /// The new tag that will be put on the lock value on drop.
    ///
    /// It can be prevent by using `cancel_tag`.
    pub fn new_tag(&self) -> VersionTag {
        self.new_tag
    }
}

impl<T: SetTag> Deref for WriteGuard<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.guard.deref().as_ref().expect("WriteGuard")
    }
}

impl<T: SetTag> DerefMut for WriteGuard<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.guard.deref_mut().as_mut().expect("WriteGuard")
    }
}

impl<T: SetTag> Drop for WriteGuard<T> {
    fn drop(&mut self) {
        if !self.cancel_tag {
            if let Some(v) = &mut *self.guard {
                v.set_tag(self.new_tag);
            }
        }
    }
}

pub struct WriteFut<T: LoadFromConn + SetTag>(State<T>);

impl<T: LoadFromConn + SetTag> WriteFut<T> {
    pub(crate) fn load(conn_or_factory: ConnOrFactory, fut: RwLockWriteFut<Option<T>>) -> Self {
        let conn = Some(conn_or_factory);
        Self(State::Write(conn, fut))
    }
}

impl<T: LoadFromConn + SetTag> Future for WriteFut<T> {
    type Item = (ConnOrFactory, WriteGuard<T>);
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let state = match &mut self.0 {
                State::Connect(guard, f) => {
                    let conn = try_ready!(f.poll());
                    let guard = guard.take().expect("Guard");

                    State::Load(Some(guard), T::load_from_conn(conn))
                }

                State::Load(guard, f) => {
                    let (conn, value) = try_ready!(f.poll());
                    let conn = ConnOrFactory::Connection(conn);
                    let mut guard = guard.take().expect("Guard");

                    *guard = Some(value);

                    let guard = WriteGuard::new(guard);
                    return Ok(Async::Ready((conn, guard)));
                }

                State::Write(conn, f) => {
                    let guard = try_ready!(f.poll().map_err(map_error));
                    let conn = conn.take().expect("ConnOrFactory");

                    if guard.is_some() {
                        let guard = WriteGuard::new(guard);
                        return Ok(Async::Ready((conn, guard)));
                    } else {
                        let guard = Some(guard);
                        match conn {
                            ConnOrFactory::Connection(c) => {
                                State::Load(guard, T::load_from_conn(c))
                            }
                            ConnOrFactory::Factory(f) => {
                                State::Connect(guard, Box::new(f.create_connection()))
                            }
                        }
                    }
                }
            };

            self.0 = state;
        }
    }
}

enum State<T: LoadFromConn> {
    Connect(
        Option<RwLockWriteGuard<Option<T>>>,
        Box<dyn Future<Item = Connection, Error = Error>>,
    ),
    Load(Option<RwLockWriteGuard<Option<T>>>, T::Future),
    Write(Option<ConnOrFactory>, RwLockWriteFut<Option<T>>),
}
