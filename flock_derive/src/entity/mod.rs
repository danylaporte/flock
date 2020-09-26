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
    let mut batch = Vec::with_capacity(5);
    let mut vars = Vec::with_capacity(fields.len());
    let mut instantiate = Vec::with_capacity(fields.len());

    fn dump_batch_single(batch: &mut Vec<(&Ident, usize)>, vars: &mut Vec<TokenStream>) {
        if !batch.is_empty() {
            for (ident, idx) in &*batch {
                let index = LitInt::new(&idx.to_string(), ident.span());
                let name = LitStr::new(&ident.to_string(), ident.span());

                vars.push(quote! {
                    let #ident = match row.get_named_err(#index, #name) {
                        Ok(v) => v,
                        Err(e) => return Err(e)
                    };
                })
            }
            batch.clear();
        }
    }

    for f in fields {
        let ident = match f.ident() {
            Ok(i) => i,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };

        if f.is_translated() {
            dump_batch_single(&mut batch, &mut vars);
            instantiate.push(quote! { #ident: Default::default(), });
        } else {
            batch.push((ident, idx));
            instantiate.push(quote! { #ident: #ident, });
            idx += 1;

            if batch.len() == 5 {
                let variables = batch.iter().map(|t| t.0);

                let names = batch
                    .iter()
                    .map(|t| LitStr::new(&t.0.to_string(), t.0.span()));

                let t = batch.first().unwrap();
                let index = LitInt::new(&t.1.to_string(), t.0.span());

                vars.push(quote! {
                    let (#(#variables,)*) = match flock::read_5_fields(row, #index, [#(#names,)*]) {
                        Ok(v) => v,
                        Err(e) => return Err(e)
                    };
                });

                batch.clear();
            }
        }
    }

    errors.result()?;
    dump_batch_single(&mut batch, &mut vars);

    Ok(quote! {
        fn new_from_row(row: &flock::mssql_client::Row) -> flock::mssql_client::Result<#ident> {
            #(#vars)*
            Ok(#ident { #(#instantiate)* })
        }
    })
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
                .add(&["(p", &p, " IS NULL OR @p", &p, " = ", &name, ")"].concat());
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

    Ok(LitStr::new(
        &["SELECT ", &select, " FROM ", &table, " WHERE ", &wheres].concat(),
        input.span(),
    ))
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
    let key_ident = quote! { (#(#key_ident,)*) };

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
            #[inline]
            fn as_ref(&self) -> &Self {
                self
            }
        }

        impl flock::AsLock for #table {
            #[inline]
            fn as_lock() -> &'static flock::Lock<Self> {
                &#lock
            }
        }

        impl flock::AsMutOpt<Self> for #table {
            #[inline]
            fn as_mut_opt(&mut self) -> Option<&mut Self> {
                Some(self)
            }
        }

        impl flock::EntityBy<#key_ty, #ident> for #table {
            #[inline]
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

                Box::pin(#table::load(table, conn, #keys_none))
            }
        }

        impl flock::SetTag for #table {
            #[inline]
            fn set_tag(&mut self, tag: flock::version_tag::VersionTag) {
                self.tag = tag;
            }
        }

        impl #table {
            #[inline]
            pub fn get(&self, #key_ident_ty) -> Option<&#ident> {
                self.map.get(&#key_ident)
            }

            #[inline]
            pub fn is_empty(&self) -> bool {
                self.map.is_empty()
            }

            #[inline]
            pub fn insert(&mut self, #key_ident_ty, value: #ident) {
                self.map.insert(#key_ident, value);
            }

            #[inline]
            pub fn iter(&self) -> std::collections::hash_map::Values<#key_ty, #ident> {
                self.map.values()
            }

            #[tracing::instrument(name = #load_name, skip(ctx, conn), err)]
            pub async fn load<'a, C>(mut ctx: C, conn: flock::ConnOrFactory, #keys_ident_opt_ty) -> flock::Result<(flock::ConnOrFactory, C)>
            where
                C: flock::AsMutOpt<#table> + 'a,
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
                    let row = Self::new_from_row(row)?;
                    ctx.map.insert(#key_row_clone, row);
                }
                Ok(ctx)
            }

            #[inline]
            pub fn len(&self) -> usize {
                self.map.len()
            }

            #new_from_row

            #[inline]
            pub fn tag(&self) -> flock::version_tag::VersionTag {
                self.tag
            }
        }

        impl<'a> std::iter::IntoIterator for &'a #table {
            type IntoIter = std::collections::hash_map::Values<'a, #key_ty, #ident>;
            type Item = &'a #ident;

            #[inline]
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

    let translated = if fields.iter().any(|f| f.is_translated()) {
        let sql_query = sql_translated_query(input)?;
        let translation_from_row = translation_from_row(input)?;

        quote! {
            let (conn, ctx) = conn
                .query_fold(#sql_query, #key_name, ctx, |mut ctx, row| {
                    #translation_from_row

                    if let Some(ctx) = ctx.as_mut_opt() {
                        translation_from_row(&mut ctx.vec, row)?;
                    }

                    Ok(ctx)
                })
                .await?;
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        #vis struct #table {
            tag: flock::version_tag::VersionTag,
            vec: flock::VecOpt<#ident>,
        }

        impl AsRef<Self> for #table {
            #[inline]
            fn as_ref(&self) -> &Self {
                self
            }
        }

        impl flock::AsLock for #table {
            #[inline]
            fn as_lock() -> &'static flock::Lock<Self> {
                &#lock
            }
        }

        impl flock::AsMutOpt<Self> for #table {
            #[inline]
            fn as_mut_opt(&mut self) -> Option<&mut Self> {
                Some(self)
            }
        }

        impl flock::EntityBy<#key_ty, #ident> for #table {
            #[inline(always)]
            fn entity_by(&self, key: #key_ty) -> Option<&#ident> {
                #table::get(self, key)
            }
        }

        impl flock::LoadFromSql for #table {
            fn load_from_sql(conn: flock::ConnOrFactory) -> flock::futures03::future::LocalBoxFuture<'static, flock::Result<(flock::ConnOrFactory, Self)>> {
                let table = Self {
                    tag: Default::default(),
                    vec: Default::default(),
                };

                Box::pin(#table::load(table, conn, None))
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
            #[inline]
            fn set_tag(&mut self, tag: flock::version_tag::VersionTag) {
                self.tag = tag;
            }
        }

        impl #table {
            #[inline]
            pub fn get(&self, #key_name: #key_ty) -> Option<&#ident> {
                self.vec.get(#key_name.into())
            }

            pub fn insert(&mut self, #key_name: #key_ty, value: #ident) {
                self.vec.insert(#key_name.into(), value);
            }

            #[inline]
            pub fn is_empty(&self) -> bool {
                self.vec.is_empty()
            }

            #[inline]
            pub fn iter(&self) -> flock::vec_opt::Iter<#ident> {
                self.vec.iter()
            }

            #[tracing::instrument(name = #load_name, skip(ctx, conn), err)]
            pub async fn load<'a, C>(mut ctx: C, conn: flock::ConnOrFactory, #key_name: Option<#key_ty>) -> flock::Result<(flock::ConnOrFactory, C)>
            where
                C: flock::AsMutOpt<#table> + 'a,
            {
                if let Some(ctx) = ctx.as_mut_opt() {
                    if let Some(id) = #key_name {
                        ctx.vec.remove(id.into());
                    } else {
                        ctx.vec.clear();
                    }
                } else {
                    return Ok((conn, ctx));
                }

                let (conn, ctx) = conn
                    .connect()
                    .await?
                    .query_fold(#sql_query, #key_name, ctx, Self::load_query_fold)
                    .await?;

                #translated

                Ok((flock::ConnOrFactory::Connection(conn), ctx))
            }

            fn load_query_fold<C>(mut ctx: C, row: &flock::mssql_client::Row) -> flock::mssql_client::Result<C>
            where
                C: flock::AsMutOpt<#table>,
            {
                if let Some(ctx) = ctx.as_mut_opt() {
                    let row = Self::new_from_row(row)?;
                    ctx.vec.insert(row.#key_name.into(), row);
                }
                Ok(ctx)
            }

            #[inline]
            pub fn len(&self) -> usize {
                self.vec.len()
            }

            #new_from_row

            #[inline]
            pub fn tag(&self) -> flock::version_tag::VersionTag {
                self.tag
            }
        }

        impl<'a> std::iter::IntoIterator for &'a #table {
            type IntoIter = flock::vec_opt::Iter<'a, #ident>;
            type Item = &'a #ident;

            #[inline]
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
        fn translation_from_row(vec: &mut flock::VecOpt<#ident>, row: &flock::mssql_client::Row) -> flock::mssql_client::Result<()> {
            let key = #key_type::try_get_from_uuid(row.get_named_err(0, #key_name)?);

            if let Some(entity) = key.and_then(|key| vec.get_mut(key.into())) {
                let culture = row.get_named_err::<translation::Culture>(1, "Culture")?;
                #fields
            }

            Ok(())
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
