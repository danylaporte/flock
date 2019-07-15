extern crate proc_macro;

use proc_macro::TokenStream;

mod lock_derive;
mod table_derive;

#[proc_macro]
pub fn locks(item: TokenStream) -> TokenStream {
    lock_derive::derive(item)
}

#[proc_macro_derive(Table)]
pub fn table(input: TokenStream) -> TokenStream {
    table_derive::derive(input)
}
