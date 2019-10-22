use super::attrs_ext::AttrsExt;
use proc_macro2::TokenStream;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Data, DeriveInput, Error, Fields, LitStr, Token, Type,
};

pub trait DeriveInputExt {
    fn input(&self) -> &DeriveInput;

    fn fields(&self) -> Result<&Fields, TokenStream> {
        let input = self.input();
        match &input.data {
            Data::Struct(s) => Ok(&s.fields),
            _ => Err(Error::new(input.span(), "Only struct are supported.").to_compile_error()),
        }
    }

    fn row_type(&self) -> Result<Type, TokenStream> {
        let input = self.input();

        input.attrs.parse_attr::<Type>("row_type")?.ok_or_else(|| {
            Error::new(input.span(), "row_type attribute expected").to_compile_error()
        })
    }

    fn table(&self) -> Result<String, TokenStream> {
        self.table_lit().map(|l| l.value())
    }

    fn table_lit(&self) -> Result<LitStr, TokenStream> {
        let input = self.input();

        let table = input.attrs.parse_attr::<LitStr>("table")?.ok_or_else(|| {
            Error::new(input.span(), "table attribute expected").to_compile_error()
        })?;

        check_table_format(&table).map_err(|e| e.to_compile_error())?;
        Ok(table)
    }

    fn translated_attr(&self) -> Result<TranslatedAttr, TokenStream> {
        let input = self.input();

        input
            .attrs
            .parse_attr::<TranslatedAttr>("translated")?
            .ok_or_else(|| {
                Error::new(input.span(), "translated attribute expected").to_compile_error()
            })
    }

    fn where_clause_attr(&self) -> Result<Option<LitStr>, TokenStream> {
        self.input().attrs.parse_attr::<LitStr>("where_clause")
    }
}

impl DeriveInputExt for DeriveInput {
    fn input(&self) -> &DeriveInput {
        self
    }
}

pub struct TranslatedAttr {
    pub key: LitStr,
    pub table: LitStr,
}

impl Parse for TranslatedAttr {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let table = input.parse()?;
        check_table_format(&table)?;

        let _separator: Token![,] = input.parse()?;
        let key = input.parse()?;

        Ok(Self { table, key })
    }
}

fn check_table_format(lit: &LitStr) -> Result<(), Error> {
    let s = lit.value();

    let e = if !s.starts_with('[') {
        "["
    } else if !s.ends_with(']') {
        "]"
    } else if !s.contains("].[") {
        "[schema].[table]"
    } else {
        return Ok(());
    };

    Err(Error::new(lit.span(), format!("expected `{}`.", e)))
}
