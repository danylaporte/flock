//! # flock
//!
//! A framework for integrating in memory mssql entities.
//!
//! ## Merge sql macros
//!
//! ```no_run
//! use flock::{MergeSql, Result, mssql_client::Connection};
//!
//! #[derive(MergeSql)]
//! #[table("[dbo].[#User]")]
//! struct User {
//!    #[key]
//!    id: i32,
//!    name: String,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let conn = Connection::from_env("DB").await?;
//!     let conn = conn.execute("CREATE TABLE #User (Id INT, name VARCHAR(50))", ()).await?;
//!     let trans = conn.transaction().await?;
//!
//!     let new_user = User {
//!         id: 1,
//!         name: "A new user".to_string(),
//!     };
//!     
//!     let _trans = new_user.merge_sql(trans).await?;
//!     Ok(())
//! }
//! ```

#![feature(const_fn)]

mod as_lock;
mod as_mut_opt;
mod as_mut_opt_wrapper;
mod conn_or_factory;
mod entity_by;
mod entity_id_set;
pub mod error;
#[doc(hidden)]
pub mod for_macros;
pub mod iter;
mod load_from_sql;
mod lock;
pub mod many_to_many;
mod merge_sql;
pub mod one_to_many;
mod read_guard;
mod read_opt_guard;
mod reset_or_reload;
pub mod result;
mod set_tag;
#[doc(hidden)]
pub mod tests;
mod try_entity_id_from_uuid;
pub mod vec_opt;
#[doc(hidden)]
pub mod version_cache;
mod write_guard;
mod write_opt_guard;

pub use as_lock::*;
pub use as_mut_opt::*;
pub use as_mut_opt_wrapper::AsMutOptWrapper;
pub use conn_or_factory::ConnOrFactory;
pub use entity_by::EntityBy;
pub use entity_id_set::EntityIdSet;
pub use error::Error;
pub use flock_derive::*;
pub use futures03;
pub use iter::FlockIter;
pub use load_from_sql::LoadFromSql;
pub use lock::Lock;
pub use many_to_many::ManyToMany;
pub use merge_sql::MergeSql;
pub use mssql_client;
pub use once_cell;
pub use once_cell::sync::OnceCell;
pub use one_to_many::OneToMany;
pub use parking_lot;
pub use rayon;
pub use read_guard::*;
pub use read_opt_guard::*;
pub use reset_or_reload::*;
pub use result::Result;
pub use serde;
pub use set_tag::SetTag;
pub use try_entity_id_from_uuid::TryEntityIdFromUuid;
pub use uuid::Uuid;
pub use vec_opt::VecOpt;
pub use version_tag;
pub use write_guard::*;
pub use write_opt_guard::*;
