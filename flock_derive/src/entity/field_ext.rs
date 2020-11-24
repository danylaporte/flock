use super::attrs_ext::AttrsExt;
use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Error, Field, Ident, LitStr, Type};

pub trait FieldExt {
    fn field(&self) -> &Field;

    fn ident(&self) -> Result<&Ident, TokenStream> {
        let f = self.field();
        f.ident
            .as_ref()
            .ok_or_else(|| Error::new(f.span(), "Ident expected.").to_compile_error())
    }

    fn column(&self) -> Result<String, TokenStream> {
        Ok(match self.field().attrs.parse_attr::<LitStr>("column")? {
            Some(c) => c.value(),
            None => self.ident()?.to_string().to_pascal_case(),
        })
    }

    fn is_identity(&self) -> bool {
        self.field()
            .attrs
            .iter()
            .any(|a| a.path.is_ident("identity"))
    }

    fn is_key(&self) -> bool {
        self.field().attrs.iter().any(|a| a.path.is_ident("key"))
    }

    fn is_string(&self) -> bool {
        let ty = &self.field().ty;
        quote!(#ty).to_string() == "String"
    }

    fn is_translated(&self) -> bool {
        let t = self.ty();
        let t = quote! { #t }.to_string();
        match t.as_str() {
            "Translated" | "translation::Translated" => true,
            _ => false,
        }
    }

    fn ty(&self) -> &Type {
        &self.field().ty
    }
}

impl FieldExt for Field {
    fn field(&self) -> &Field {
        self
    }
}
