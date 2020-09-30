#[macro_use]
mod macros;

mod attrs_ext;
mod derive_input_ext;
mod errors;
mod field_ext;
mod merge_sql;
mod sql_string_ext;

use derive_input_ext::DeriveInputExt;
use errors::Errors;
use field_ext::FieldExt;
use inflector::Inflector;
pub(crate) use merge_sql::merge;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use sql_string_ext::SqlStringExt;
use syn::{spanned::Spanned, DeriveInput, Error, Ident, LitBool, LitInt, LitStr};

pub fn generate(input: DeriveInput) -> TokenStream {
    let table = try_token_stream!(table(&input));
    let test_load = test_load(&input);
    let test_schema = try_token_stream!(test_schema(&input));
    let test_schema_translated = try_token_stream!(test_schema_translated(&input));

    quote! {
        #table

        #[cfg(test)]
        mod tests {
            use super::*;

            #test_load
            #test_schema
            #test_schema_translated
        }
    }
}

fn new_from_row(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    let fields = input.fields()?;
    let ident = &input.ident;
    let mut errors = Vec::new();
    let mut idx = 0;
    let mut instantiate = Vec::with_capacity(fields.len());

    for f in fields {
        let ident = match f.ident() {
            Ok(i) => i,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };

        if f.is_translated() {
            instantiate.push(quote! { #ident: Default::default(), });
        } else {
            let span = ident.span();
            let index = LitInt::new(&idx.to_string(), span);
            let name = LitStr::new(&ident.to_string(), span);

            instantiate.push(quote! {
                #ident: match row.get_named_err(#index, #name) {
                    Ok(v) => v,
                    Err(e) => return Err(e)
                },
            });

            idx += 1;
        }
    }

    errors.result()?;
    Ok(quote! { #ident { #(#instantiate)* } })
}

fn sql_query(input: &DeriveInput) -> Result<LitStr, TokenStream> {
    let fields = input.fields()?;
    let mut errors = Vec::new();
    let mut select = String::with_capacity(1000);
    let mut wheres = String::with_capacity(250);
    let mut idx = 1;

    for f in fields {
        if f.is_translated() {
            continue;
        }

        let name = match f.column() {
            Ok(mut n) => {
                n.insert(0, '[');
                n.push(']');
                n
            }
            Err(e) => {
                errors.push(e);
                continue;
            }
        };

        select.add_sep(",").add(&name);

        if f.is_key() {
            let p = idx.to_string();

            wheres
                .add_sep(" AND ")
                .add("(@p")
                .add(&p)
                .add(" IS NULL OR @p")
                .add(&p)
                .add(" = ")
                .add(&name)
                .add(")");

            idx += 1;
        }
    }

    let table = input.table().map_err(|e| errors.push(e));

    match input.where_clause_attr() {
        Ok(Some(w)) => {
            wheres.add_sep(" AND ").add(&w.value());
        }
        Ok(None) => {}
        Err(e) => errors.push(e),
    }

    errors.result()?;

    let table = table.expect("table");
    let sql = ["SELECT ", &select, " FROM ", &table, " WHERE ", &wheres].concat();

    Ok(LitStr::new(&sql, input.span()))
}

fn sql_translated_query(input: &DeriveInput) -> Result<LitStr, TokenStream> {
    let fields = input.fields()?;
    let mut errors = Vec::new();

    let select = fields
        .iter()
        .filter(|f| f.is_translated())
        .filter_map(|f| f.column().map_err(|e| errors.push(e)).ok())
        .map(|mut v| {
            v.insert(0, '[');
            v.push(']');
            v
        })
        .collect::<Vec<_>>()
        .join(",");

    let attr = input.translated_attr().map_err(|e| errors.push(e));

    errors.result()?;

    let attr = attr.expect("table_key");

    Ok(LitStr::new(
        &format!(
            "SELECT {key},Culture,{select} FROM {table} WHERE (@p1 IS NULL OR @p1 = {key})",
            select = select,
            table = attr.table.value(),
            key = attr.key.value(),
        ),
        input.span(),
    ))
}

fn table(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    let fields = input.fields()?;
    let count = fields.iter().filter(|f| f.is_key()).count();

    match count {
        0 => Err(
            Error::new(input.span(), "Entity must have at least on key field.").to_compile_error(),
        ),
        1 if !fields.iter().any(|f| f.is_key() && f.is_string()) => table_single_key(input),
        _ => table_multi_key(input),
    }
}

fn table_multi_key(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    let ident = &input.ident;
    let ident_str = ident.to_string();
    let table_name = ident_str.to_plural().to_pascal_case();
    let table = Ident::new(&table_name, ident.span());
    let fields = input.fields()?;
    let lock = Ident::new(&ident_str.to_screaming_snake_case(), ident.span());
    let vis = &input.vis;
    let new_from_row = new_from_row(input)?;

    let mut load_name = table_name.clone();
    load_name.push_str("::load");

    let load_name = LitStr::new(&load_name, ident.span());

    errors::check_entity_must_be_singular(&ident_str, &table_name, ident.span())?;

    let key_ident_ty = fields.iter().filter(|f| f.is_key()).map(|f| {
        let id = &f.ident;
        let ty = &f.ty;
        quote! { #id: #ty }
    });
    let key_ident_ty = quote! { #(#key_ident_ty),* };

    let keys_ident_opt_ty = fields.iter().filter(|f| f.is_key()).map(|f| {
        let id = &f.ident;
        let ty = &f.ty;
        quote! { #id: Option<#ty> }
    });
    let keys_ident_opt_ty = quote! { #(#keys_ident_opt_ty),* };

    let key_ident = fields
        .iter()
        .filter(|f| f.is_key())
        .filter_map(|f| f.ident.as_ref());
    let key_sep_ident = quote! { #(#key_ident,)* };
    let key_ident = quote! { (#key_sep_ident) };

    let key_ty = fields.iter().filter(|f| f.is_key()).map(|f| &f.ty);
    let key_ty = quote! { (#(#key_ty,)*) };

    let key_row_clone = fields.iter().filter(|f| f.is_key()).map(|f| {
        let f = f.ident.as_ref();
        quote! { row.#f.clone() }
    });
    let key_row_clone = quote! { (#(#key_row_clone,)*) };

    let is_all_keys_none = fields.iter().filter(|f| f.is_key()).map(|f| {
        let f = f.ident.as_ref();
        quote! { #f.is_none() }
    });
    let is_all_keys_none = quote! { #(#is_all_keys_none)&&* };

    let retain_keys = fields
        .iter()
        .filter(|f| f.is_key())
        .enumerate()
        .map(|(i, f)| {
            let f = f.ident.as_ref();
            let i = LitInt::new(&i.to_string(), Span::call_site());
            quote! { #f.as_ref().map_or(false, |k| k != &keys.#i )}
        });
    let retain_keys = quote! { #(#retain_keys)||* };

    let keys_none = fields
        .iter()
        .filter(|f| f.is_key())
        .map(|_| quote! { None });

    let keys_none = quote! { #(#keys_none),* };
    let sql_query = sql_query(input)?;

    Ok(quote! {
        #vis struct #table {
            map: std::collections::HashMap<#key_ty, #ident>,
            tag: flock::version_tag::VersionTag,
        }

        impl AsRef<Self> for #table {
            fn as_ref(&self) -> &Self {
                self
            }
        }

        impl flock::AsLock for #table {
            fn as_lock() -> &'static flock::Lock<Self> {
                &#lock
            }
        }

        impl flock::AsMutOpt<Self> for #table {
            fn as_mut_opt(&mut self) -> Option<&mut Self> {
                Some(self)
            }
        }

        impl flock::EntityBy<#key_ty, #ident> for #table {
            fn entity_by(&self, #key_ident: #key_ty) -> Option<&#ident> {
                self.map.get(&#key_ident)
            }
        }

        impl flock::LoadFromSql for #table {
            fn load_from_sql(conn: flock::ConnOrFactory) -> flock::futures03::future::LocalBoxFuture<'static, flock::Result<(flock::ConnOrFactory, Self)>> {
                let table = Self {
                    map: Default::default(),
                    tag: Default::default(),
                };

                Box::pin(Self::load(table, conn, #keys_none))
            }
        }

        impl flock::SetTag for #table {
            fn set_tag(&mut self, tag: flock::version_tag::VersionTag) {
                self.tag = tag;
            }
        }

        impl #table {
            pub fn get(&self, #key_ident_ty) -> Option<&#ident> {
                self.map.get(&#key_ident)
            }

            pub fn is_empty(&self) -> bool {
                self.map.is_empty()
            }

            pub fn insert(&mut self, #key_ident_ty, value: #ident) {
                self.map.insert(#key_ident, value);
            }

            pub fn iter(&self) -> std::collections::hash_map::Values<#key_ty, #ident> {
                self.map.values()
            }

            #[tracing::instrument(name = #load_name, skip(ctx, conn), err)]
            pub async fn load<'a, C>(mut ctx: C, conn: flock::ConnOrFactory, #keys_ident_opt_ty) -> flock::Result<(flock::ConnOrFactory, C)>
            where
                C: flock::AsMutOpt<#table> + 'a
            {
                if let Some(ctx) = ctx.as_mut_opt() {
                    if #is_all_keys_none {
                        ctx.map.clear();
                    } else {
                        ctx.map.retain(|keys, _| { #retain_keys });
                    }
                } else {
                    return Ok((conn, ctx))
                }

                match conn.connect().await {
                    Ok(conn) => match conn.query_fold(#sql_query, #key_ident, ctx, Self::load_query_fold).await {
                        Ok((conn, ctx)) => Ok((flock::ConnOrFactory::Connection(conn), ctx)),
                        Err(e) => Err(e.into()),
                    },
                    Err(e) => Err(e),
                }
            }

            fn load_query_fold<C>(mut ctx: C, row: &flock::mssql_client::Row) -> flock::mssql_client::Result<C>
            where
                C: flock::AsMutOpt<#table>,
            {
                if let Some(ctx) = ctx.as_mut_opt() {
                    let row = #new_from_row;
                    ctx.map.insert(#key_row_clone, row);
                }
                Ok(ctx)
            }

            pub fn len(&self) -> usize {
                self.map.len()
            }

            pub fn tag(&self) -> flock::version_tag::VersionTag {
                self.tag
            }
        }

        impl<'a> std::iter::IntoIterator for &'a #table {
            type IntoIter = std::collections::hash_map::Values<'a, #key_ty, #ident>;
            type Item = &'a #ident;

            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        static #lock: flock::Lock<#table> = flock::Lock::new();
    })
}

fn table_single_key(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    let ident = &input.ident;
    let ident_str = ident.to_string();
    let table_name = ident_str.to_plural().to_pascal_case();
    let table = Ident::new(&table_name, ident.span());
    let fields = input.fields()?;
    let new_from_row = new_from_row(input)?;
    let sql_query = sql_query(input)?;
    let lock = Ident::new(&ident_str.to_screaming_snake_case(), ident.span());
    let vis = &input.vis;
    let mut load_name = table_name.clone();

    load_name.push_str("::load");

    let load_name = LitStr::new(&load_name, ident.span());

    errors::check_entity_must_be_singular(&ident_str, &table_name, ident.span())?;

    let (key_name, key_ty) = fields
        .iter()
        .find(|f| f.is_key())
        .and_then(|f| Some((f.ident.as_ref()?, &f.ty)))
        .ok_or_else(|| errors::missing_key(input.span()))?;

    let load_impl = if fields.iter().any(|f| f.is_translated()) {
        let translated_sql_query = sql_translated_query(input)?;
        let translation_from_row = translation_from_row(input)?;

        quote! {
            flock::for_macros::load_single_key_translate(conn, &mut table.vec, key, #sql_query, #translated_sql_query, |vec, row| {
                let row = #new_from_row;
                vec.insert(row.#key_name.into(), row);
                Ok(vec)
            }, #translation_from_row).await?
        }
    } else {
        quote! {
            flock::for_macros::load_single_key(conn, &mut table.vec, key, #sql_query, |vec, row| {
                let row = #new_from_row;
                vec.insert(row.#key_name.into(), row);
                Ok(vec)
            }).await?
        }
    };

    Ok(quote! {
        #vis struct #table {
            tag: flock::version_tag::VersionTag,
            vec: flock::VecOpt<#ident>,
        }

        impl AsRef<Self> for #table {
            fn as_ref(&self) -> &Self {
                self
            }
        }

        impl flock::AsLock for #table {
            fn as_lock() -> &'static flock::Lock<Self> {
                &#lock
            }
        }

        impl flock::AsMutOpt<Self> for #table {
            fn as_mut_opt(&mut self) -> Option<&mut Self> {
                Some(self)
            }
        }

        impl flock::EntityBy<#key_ty, #ident> for #table {
            fn entity_by(&self, key: #key_ty) -> Option<&#ident> {
                self.get(key)
            }
        }

        impl flock::LoadFromSql for #table {
            fn load_from_sql(conn: flock::ConnOrFactory) -> flock::futures03::future::LocalBoxFuture<'static, flock::Result<(flock::ConnOrFactory, Self)>> {
                let table = Self {
                    tag: Default::default(),
                    vec: Default::default(),
                };

                Box::pin(Self::load(table, conn, None))
            }
        }

        impl flock::ResetOrReload<Option<#key_ty>> for #table {
            fn reset_or_reload(
                fac: flock::ConnOrFactory,
                key: Option<#key_ty>,
            ) -> flock::futures03::future::LocalBoxFuture<'static, flock::Result<flock::ConnOrFactory>>
            {
                Box::pin(async move {
                    let mut guard = <Self as flock::AsLock>::as_lock().write_opt().await;

                    if guard.is_some() {
                        Ok(Self::load(guard, fac, key).await?.0)
                    } else {
                        *guard = None;
                        Ok(fac)
                    }
                })
            }
        }

        impl flock::SetTag for #table {
            fn set_tag(&mut self, tag: flock::version_tag::VersionTag) {
                self.tag = tag;
            }
        }

        impl #table {
            pub fn get(&self, #key_name: #key_ty) -> Option<&#ident> {
                self.vec.get(#key_name.into())
            }

            pub fn insert(&mut self, #key_name: #key_ty, value: #ident) {
                self.vec.insert(#key_name.into(), value);
            }

            pub fn is_empty(&self) -> bool {
                self.vec.is_empty()
            }

            pub fn iter(&self) -> flock::vec_opt::Iter<#ident> {
                self.vec.iter()
            }

            #[tracing::instrument(name = #load_name, skip(ctx, conn), err)]
            pub async fn load<'a, C>(mut ctx: C, mut conn: flock::ConnOrFactory, key: Option<#key_ty>) -> flock::Result<(flock::ConnOrFactory, C)>
            where
                C: flock::AsMutOpt<#table> + 'a,
            {
                if let Some(table) = ctx.as_mut_opt() {
                    conn = #load_impl;
                }

                Ok((conn, ctx))
            }

            pub fn len(&self) -> usize {
                self.vec.len()
            }

            pub fn tag(&self) -> flock::version_tag::VersionTag {
                self.tag
            }
        }

        impl<'a> std::iter::IntoIterator for &'a #table {
            type IntoIter = flock::vec_opt::Iter<'a, #ident>;
            type Item = &'a #ident;

            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        static #lock: flock::Lock<#table> = flock::Lock::new();
    })
}

fn translation_from_row(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    let fields = input.fields()?;
    let mut errors = Vec::new();

    let (key_name, key_type) = fields
        .iter()
        .find(|f| f.is_key())
        .and_then(|f| {
            let ident = f.ident().map_err(|e| errors.push(e)).ok()?;
            let name = LitStr::new(&ident.to_string(), ident.span());
            let ty = &f.ty;
            Some((name, ty))
        })
        .ok_or_else(|| errors::missing_key(input.span()))?;

    let fields = fields
        .iter()
        .filter(|f| f.is_translated())
        .enumerate()
        .filter_map(|(index, f)| {
            // add 2 to idx because the first 2 fields are the key and culture.
            let index = index + 2;

            let ident = f.ident().map_err(|e| errors.push(e)).ok()?;
            let index = LitInt::new(&index.to_string(), f.span());
            let name = LitStr::new(&ident.to_string(), ident.span());

            Some(quote! {
                if let Some(v) = row.get_named_err::<Option<&str>>(#index, #name)? {
                    entity.#ident.set(culture, v);
                }
            })
        });

    let fields = quote! { #(#fields)* };
    let ident = &input.ident;

    errors.result()?;

    Ok(quote! {
        |vec: &mut flock::VecOpt<#ident>, row: &flock::mssql_client::Row| {
            let key = #key_type::try_get_from_uuid(row.get_named_err(0, #key_name)?);

            if let Some(entity) = key.and_then(|key| vec.get_mut(key.into())) {
                let culture = row.get_named_err::<translation::Culture>(1, "Culture")?;
                #fields
            }

            Ok(vec)
        }
    })
}

fn test_load(input: &DeriveInput) -> TokenStream {
    let ident = &input.ident;
    let ident_str = ident.to_string().to_plural();
    let table = Ident::new(&ident_str.to_pascal_case(), ident.span());

    let mut fn_ident = ident_str.to_snake_case();
    fn_ident.insert_str(0, "test_load_");

    let fn_ident = Ident::new(&fn_ident, input.span());

    quote! {
        #[tokio::test]
        async fn #fn_ident() {
            flock::tests::test_load::<#table>().await;
        }
    }
}

fn test_schema(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    let fields = input.fields()?;
    let mut errors = Vec::new();
    let ident = &input.ident;
    let table_lit = input.table_lit().map_err(|e| errors.push(e));

    let mut fn_ident = ident.to_string().to_snake_case();
    fn_ident.insert_str(0, "test_schema_");

    let fn_ident = Ident::new(&fn_ident, input.span());

    let fields = fields.iter().filter_map(|f| {
        if f.is_translated() {
            return None;
        }

        let ident = f.ident().map_err(|e| errors.push(e)).ok()?;
        let column = f.column().map_err(|e| errors.push(e)).ok()?;
        let name = LitStr::new(&column.to_lowercase(), f.span());
        let ty = f.ty();

        let is_key = LitBool {
            value: f.is_key(),
            span: ident.span(),
        };

        Some(quote! {
            (
                #name,
                &<<#ty as FromColumn>::Value as SqlValue>::check_db_ty,
                Some(<<#ty as FromColumn>::Value as SqlValue>::is_nullable()),
                #is_key
            )
        })
    });

    let fields = quote! { #(#fields,)* };

    errors.result()?;

    let table = table_lit.expect("table");

    Ok(quote! {
        #[tokio::test]
        async fn #fn_ident() {
            use flock::mssql_client::{FromColumn, SqlValue};
            flock::tests::test_schema(#table, &[#fields]).await;
        }
    })
}

fn test_schema_translated(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    let fields = input.fields()?;

    if !fields.iter().any(|f| f.is_translated()) {
        return Ok(quote! {});
    }

    let mut errors = Vec::new();
    let ident = &input.ident;
    let attr = input.translated_attr().map_err(|e| errors.push(e));

    let fn_ident = Ident::new(
        &format!("test_schema_{}_translated", ident).to_snake_case(),
        input.span(),
    );

    let fields = fields.iter().filter(|f| f.is_translated()).filter_map(|f| {
        let column = f.column().map_err(|e| errors.push(e)).ok()?;
        let column = LitStr::new(&column.to_lowercase(), f.span());

        Some(quote! {
            (
                #column,
                &<<&str as FromColumn>::Value as SqlValue>::check_db_ty,
                None,
                false,
            )
        })
    });

    let fields = quote! { #(#fields,)* };

    errors.result()?;

    let attr = attr.expect("attr");
    let key = attr.key;
    let key = LitStr::new(&key.value().to_lowercase(), key.span());
    let table = attr.table;

    Ok(quote! {
        #[tokio::test]
        async fn #fn_ident() {
            use flock::mssql_client::{FromColumn, SqlValue};
            flock::tests::test_schema(#table, &[
                (#key, &<<flock::Uuid as FromColumn>::Value as SqlValue>::check_db_ty, Some(false), true),
                ("culture", &<<&str as FromColumn>::Value as SqlValue>::check_db_ty, Some(false), true),
                #fields
            ]).await;
        }
    })
}
