extern crate proc_macro;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod entity_id;

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
