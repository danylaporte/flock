pub extern crate lazy_static;

mod as_lock;
mod as_mut_opt;
mod as_mut_opt_wrapper;
mod conn_or_factory;
mod entity_by;
mod entity_id_set;
pub mod iter;
mod load_from_conn;
mod lock;
mod lock_states;
pub mod many_to_many;
pub mod one_to_many;
mod read;
mod read_opt;
mod set_tag;
#[doc(hidden)]
pub mod tests;
mod vec_opt;
#[doc(hidden)]
pub mod version_cache;
mod write;
mod write_opt;

pub use as_lock::AsLock;
pub use as_mut_opt::*;
pub use as_mut_opt_wrapper::AsMutOptWrapper;
pub use conn_or_factory::ConnOrFactory;
pub use entity_by::EntityBy;
pub use entity_id_set::EntityIdSet;
pub use failure;
pub use futures;
pub use iter::FlockIter;
pub use load_from_conn::{LoadFromConn, LoadFromConnFut};
pub use lock::Lock;
pub use lock_states::LockStates;
pub use log;
pub use many_to_many::ManyToMany;
pub use mssql_client;
pub use once_cell::sync::OnceCell;
pub use one_to_many::OneToMany;
pub use read::{ReadFut, ReadGuard};
pub use read_opt::{ReadOptFut, ReadOptGuard};
pub use serde;
pub use set_tag::SetTag;
pub use tokio;
pub use uuid::Uuid;
pub use vec_opt::VecOpt;
pub use version_tag;
pub use write::{WriteFut, WriteGuard};
pub use write_opt::{WriteOptFut, WriteOptGuard};

fn map_error<T>(_: T) -> failure::Error {
    failure::format_err!("Lock Error")
}
