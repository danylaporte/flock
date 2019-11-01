mod attrs_ext;
mod derive_input_ext;
mod errors;
mod field_ext;

#[macro_use]
mod macros;

use derive_input_ext::DeriveInputExt;
use errors::Errors;
use field_ext::FieldExt;
use inflector::Inflector;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{spanned::Spanned, DeriveInput, Error, Ident, LitBool, LitInt, LitStr};

pub fn generate(input: DeriveInput) -> TokenStream {
    let table = try_token_stream!(table(&input));
    let test_load = test_load(&input);
    let test_schema = try_token_stream!(test_schema(&input));
    let test_schema_translated = try_token_stream!(test_schema_translated(&input));

    quote! {
        #table
        #test_load
        #test_schema
        #test_schema_translated
    }
}

fn new_from_row(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    let fields = input.fields()?;
    let ident = &input.ident;
    let mut errors = Vec::new();
    let mut idx = 0;

    let fields = fields.iter().filter_map(|f| {
        let ident = f.ident().map_err(|e| errors.push(e)).ok()?;

        if f.is_translated() {
            return Some(quote! { #ident: Default::default() });
        }

        let index = LitInt::new(&idx.to_string(), f.span());
        let name = LitStr::new(&ident.to_string(), ident.span());

        idx += 1;

        Some(quote! {
            #ident: row
                .get(#index)
                .and_then(flock::mssql_client::FromColumn::from_column)
                .context(#name)?
        })
    });

    let fields = quote! { #(#fields,)* };

    errors.result()?;

    Ok(quote! {
        fn new_from_row(row: &flock::mssql_client::Row) -> Result<#ident, flock::failure::Error> {
            Ok(#ident { #fields })
        }
    })
}

fn sql_query(input: &DeriveInput) -> Result<LitStr, TokenStream> {
    let fields = input.fields()?;
    let mut errors = Vec::new();

    let select = fields
        .iter()
        .filter(|f| !f.is_translated())
        .filter_map(|f| f.column().map_err(|e| errors.push(e)).ok())
        .map(|v| format!("[{}]", v))
        .collect::<Vec<_>>()
        .join(",");

    let table = input.table().map_err(|e| errors.push(e));
    let where_clause = input.where_clause_attr().map_err(|e| errors.push(e));

    errors.result()?;

    let table = table.expect("table");
    let where_clause = where_clause.expect("where_clause");

    let where_clause = fields
        .iter()
        .filter(|f| f.is_key())
        .filter_map(|f| f.column().map_err(|e| errors.push(e)).ok())
        .enumerate()
        .map(|(i, v)| {
            format!(
                "(@p{p} IS NULL OR @p{p} = [{field}])",
                p = (i + 1),
                field = v
            )
        })
        .chain(where_clause.into_iter().map(|t| t.value()))
        .collect::<Vec<_>>()
        .join(" AND ");

    errors.result()?;

    Ok(LitStr::new(
        &format!(
            "SELECT {select} FROM {table} WHERE {where_clause}",
            select = select,
            table = table,
            where_clause = where_clause,
        ),
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
        .map(|v| format!("[{}]", v))
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
    match input.fields()?.iter().filter(|f| f.is_key()).count() {
        0 => Err(
            Error::new(input.span(), "Entity must have at least on key field.").to_compile_error(),
        ),
        1 => table_single_key(input),
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
    let context = LitStr::new(&ident_str, ident.span());
    let new_from_row = new_from_row(input)?;

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
    let key_row_clone = quote! { (#(#key_row_clone),*) };

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

        impl flock::LoadFromConn for #table {
            fn load_from_conn(conn: flock::mssql_client::Connection) -> flock::LoadFromConnFut<Self> {
                let table = Self {
                    map: Default::default(),
                    tag: Default::default(),
                };

                Box::new(#table::load(table, conn, #keys_none))
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

            pub fn iter(&self) -> impl Iterator<Item = &#ident> {
                self.map.values()
            }

            pub fn load<C>(mut ctx: C, conn: flock::mssql_client::Connection, #keys_ident_opt_ty) -> impl flock::futures::Future<
                Item = (flock::mssql_client::Connection, C),
                Error = flock::failure::Error
            >
            where
                C: flock::AsMutOpt<#table> + 'static,
            {
                use flock::{failure::ResultExt, futures::future::{lazy, ok, Either, Future}};

                #new_from_row

                if let Some(ctx) = ctx.as_mut_opt() {
                    if #is_all_keys_none {
                        ctx.map.clear();
                    } else {
                        ctx.map.retain(|keys, _| { #retain_keys })
                    }
                } else {
                    return Either::A(ok((conn, ctx)));
                }

                let lazy = lazy(|| {
                    flock::log::trace!("Loading");
                    ok::<_, flock::failure::Error>(())
                });

                Either::B(lazy.and_then(move |_| conn.query_fold(#sql_query, #key_ident, ctx, |mut ctx, row| {
                    if let Some(ctx) = ctx.as_mut_opt() {
                        let row = new_from_row(row).context(#context)?;
                        ctx.map.insert(#key_row_clone, row);
                    }
                    Ok(ctx)
                })).inspect(|_| flock::log::trace!("Loaded")))
            }

            pub fn len(&self) -> usize {
                self.map.len()
            }

            pub fn tag(&self) -> flock::version_tag::VersionTag {
                self.tag
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
    let context = LitStr::new(&ident_str, ident.span());
    let lock = Ident::new(&ident_str.to_screaming_snake_case(), ident.span());
    let vis = &input.vis;

    errors::check_entity_must_be_singular(&ident_str, &table_name, ident.span())?;

    let (key_name, key_ty) = fields
        .iter()
        .find(|f| f.is_key())
        .and_then(|f| Some((f.ident.as_ref()?, &f.ty)))
        .ok_or_else(|| errors::missing_key(input.span()))?;

    let translated = if fields.iter().any(|f| f.is_translated()) {
        let sql_query = sql_translated_query(input)?;
        let translation_from_row = translation_from_row(input)?;
        let context = LitStr::new(&format!("{} translated", ident_str), ident.span());

        quote! {
            .and_then(move |(conn, ctx)| conn
                .query_fold(#sql_query, #key_name, ctx, |mut ctx, row| {
                    #translation_from_row

                    if let Some(ctx) = ctx.as_mut_opt() {
                        translation_from_row(&mut ctx.vec, row).context(#context)?;
                    }

                    Ok(ctx)
                })
            )
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
                #table::get(self, key)
            }
        }

        impl flock::LoadFromConn for #table {
            fn load_from_conn(conn: flock::mssql_client::Connection) -> flock::LoadFromConnFut<Self> {
                let table = Self {
                    tag: Default::default(),
                    vec: Default::default(),
                };

                Box::new(#table::load(table, conn, None))
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

            pub fn is_empty(&self) -> bool {
                self.vec.is_empty()
            }

            pub fn iter(&self) -> impl Iterator<Item = &#ident> {
                self.vec.iter()
            }

            pub fn load<C>(mut ctx: C, conn: flock::mssql_client::Connection, #key_name: Option<#key_ty>) -> impl flock::futures::Future<
                Item = (flock::mssql_client::Connection, C),
                Error = flock::failure::Error
            >
            where
                C: flock::AsMutOpt<#table> + 'static,
            {
                use flock::{failure::ResultExt, futures::future::{lazy, ok, Either, Future}};

                #new_from_row

                if let Some(ctx) = ctx.as_mut_opt() {
                    if let Some(id) = #key_name {
                        ctx.vec.remove(id.into());
                    } else {
                        ctx.vec.clear();
                    }
                } else {
                    return Either::A(ok((conn, ctx)));
                }

                let lazy = lazy(|| {
                    flock::log::trace!("Loading");
                    ok::<_, flock::failure::Error>(())
                });

                Either::B(lazy.and_then(move |_| conn.query_fold(#sql_query, #key_name, ctx, |mut ctx, row| {
                    if let Some(ctx) = ctx.as_mut_opt() {
                        let row = new_from_row(row).context(#context)?;
                        ctx.vec.insert(row.#key_name.into(), row);
                    }
                    Ok(ctx)
                }))#translated.inspect(|_| flock::log::trace!("Loaded")))
            }

            pub fn len(&self) -> usize {
                self.vec.len()
            }

            pub fn tag(&self) -> flock::version_tag::VersionTag {
                self.tag
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
                if let Some(v) = row.get(#index).context(#name)? {
                    entity.#ident[culture] = flock::mssql_client::FromColumn::from_column(v).context(#name)?;
                }
            })
        });

    let fields = quote! { #(#fields)* };
    let ident = &input.ident;

    errors.result()?;

    Ok(quote! {
        fn translation_from_row(vec: &mut flock::VecOpt<#ident>, row: &flock::mssql_client::Row) -> Result<(), flock::failure::Error> {
            use flock::failure::ResultExt;

            let key = #key_type::try_get_from_uuid(row.get(0).context(#key_name)?);

            if let Some(entity) = key.and_then(|key| vec.get_mut(key.into())) {
                let culture = row.get::<translation::Culture>(1).context("culture")?;
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

    let fn_ident = Ident::new(
        &format!("test_load_{}", ident_str.to_snake_case()),
        input.span(),
    );

    quote! {
        #[test]
        fn #fn_ident() {
            flock::tests::test_load::<#table>();
        }
    }
}

fn test_schema(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    let fields = input.fields()?;
    let mut errors = Vec::new();
    let ident = &input.ident;
    let table_lit = input.table_lit().map_err(|e| errors.push(e));

    let fn_ident = Ident::new(
        &format!("test_schema_{}", ident).to_snake_case(),
        input.span(),
    );

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
        #[test]
        fn #fn_ident() {
            use flock::mssql_client::{FromColumn, SqlValue};
            flock::tests::test_schema(#table, &[#fields]);
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
        #[test]
        fn #fn_ident() {
            use flock::mssql_client::{FromColumn, SqlValue};
            flock::tests::test_schema(#table, &[
                (#key, &<<flock::Uuid as FromColumn>::Value as SqlValue>::check_db_ty, Some(false), true),
                ("culture", &<<&str as FromColumn>::Value as SqlValue>::check_db_ty, Some(false), true),
                #fields
            ]);
        }
    })
}
