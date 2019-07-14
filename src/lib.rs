mod conn_or_factory;
mod guard;
mod load_from_conn;
mod lock;
mod read;
mod read_opt;
mod write;
mod write_opt;

pub(crate) use self::conn_or_factory::ConnOrFactory;

pub use self::guard::WGuard;
pub use self::load_from_conn::LoadFromConn;
pub use self::lock::Lock;
pub use self::read::{ReadFut, ReadGuard};
pub use self::read_opt::{ReadOptFut, ReadOptGuard};
pub use self::write::{WriteFut, WriteGuard};
pub use self::write_opt::{WriteOptFut, WriteOptGuard};

type LockValue<T> = (Option<T>, version_tag::VersionTag);

fn map_error<T>(_: T) -> failure::Error {
    failure::format_err!("Lock Error")
}
