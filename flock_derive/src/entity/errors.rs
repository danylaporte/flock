use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Error;

pub trait Errors {
    fn result(&self) -> Result<(), TokenStream>;
}

impl Errors for Vec<TokenStream> {
    fn result(&self) -> Result<(), TokenStream> {
        if self.is_empty() {
            Ok(())
        } else {
            let v = self;
            Err(quote! { #(#v)* })
        }
    }
}

pub fn check_entity_must_be_singular(
    entity: &str,
    table: &str,
    span: Span,
) -> Result<(), TokenStream> {
    if entity == table {
        Err(Error::new(span, "Entity must be singular").to_compile_error())
    } else {
        Ok(())
    }
}

pub fn missing_key(span: Span) -> TokenStream {
    Error::new(span, "Missing key").to_compile_error()
}
