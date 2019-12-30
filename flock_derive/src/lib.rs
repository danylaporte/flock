extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, ItemTrait};

mod entity;
mod entity_id;
mod lock_derive;
mod relations;

/// Turn a struct into an entity, implementing the load from the db and
/// creating a table for storing it.
///
/// # Example
///
/// Load a simple table
/// ```
/// use flock_derive::{Entity, EntityId};
///
/// #[derive(Entity)]
/// #[table("[dbo].[Accounts]")]
/// #[where_clause("[Name] IS NOT NULL")]
/// pub struct Account {
///     #[key]
///     pub id: AccountId,
///     pub name: String,
///     pub address: Option<String>,
/// }
///
/// #[derive(EntityId)]
/// pub struct AccountId(u32);
/// ```
///
/// Load a multi-key table
/// ```
/// use flock_derive::{Entity, EntityId};
///
/// #[derive(Entity)]
/// #[table("[dbo].[UserAccounts]")]
/// pub struct UserAccount {
///     #[key]
///     user_id: UserId,
///     #[key]
///     account_id: AccountId,
/// }
///
/// #[derive(EntityId)]
/// pub struct AccountId(u32);
///
/// #[derive(EntityId)]
/// pub struct UserId(u32);
/// ```
#[proc_macro_derive(Entity, attributes(column, key, table, translated, where_clause))]
pub fn entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    entity::generate(input).into()
}

/// Transform a tuple into an EntityId.
///
/// An entity id is used to map a Uuid into a smaller integer type.
/// This improve performance in table because it is used as a direct pointer.
///
/// ```
/// use flock_derive::EntityId;
/// use uuid::Uuid;
///
/// #[derive(EntityId)]
/// pub struct AccountId(u32);
///
/// // InvoiceId as an indexing space of u64
/// #[derive(EntityId)]
/// pub struct InvoiceId(u64);
///
/// fn main() {
///     let id = Uuid::new_v4();
///
///     // Transform a uuid into an AccountId
///     let account_id = AccountId::from(id);
///
///     assert_eq!(account_id, id);
/// }
/// ```
#[proc_macro_derive(EntityId)]
pub fn entity_id(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    entity_id::generate(input).into()
}

#[proc_macro]
pub fn locks(item: TokenStream) -> TokenStream {
    lock_derive::locks(item)
}

#[proc_macro]
pub fn locks_await(item: TokenStream) -> TokenStream {
    lock_derive::locks_await(item)
}

#[proc_macro_attribute]
pub fn relations(_args: TokenStream, input: TokenStream) -> TokenStream {
    let f = parse_macro_input!(input as ItemTrait);
    relations::generate(f).into()
}
