use super::{DeriveInputExt, Errors, FieldExt, SqlStringExt};
use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use std::iter::once;
use syn::{spanned::Spanned, DeriveInput, Error, Field, Ident, LitBool, LitInt, LitStr};

pub(crate) fn merge(input: &DeriveInput) -> TokenStream {
    try_token_stream!(impl_merge(input))
}

fn impl_merge(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    let ident = &input.ident;
    let fields = input.fields()?;

    let standard = fields
        .iter()
        .filter(|f| !f.is_translated())
        .map(MergeField::Field);

    let reload_on_write = input.reload_on_write();
    let execute = merge_execute(&input, standard, &input.table()?, reload_on_write)?;
    let standard = quote! { trans = trans.#execute; };
    let translated = fields.iter().filter(|f| f.is_key() || f.is_translated());

    let translated = if translated.clone().any(|f| f.is_translated()) {
        let table = input.translated_attr()?.table;

        let fields = translated
            .map(MergeField::Field)
            .chain(once(MergeField::Culture));

        let execute = merge_execute(&input, fields, &table.value(), false)?;

        quote! {
            for &culture in &Culture::DB_CULTURES {
                trans = trans.#execute;
            }
        }
    } else {
        quote! {}
    };

    let test_merge_sql_schema = test_merge_sql_schema(input)?;

    Ok(quote! {
        impl flock::MergeSql for #ident {
            fn merge_sql<'a>(
                &'a mut self,
                mut trans: flock::mssql_client::Transaction,
            ) -> flock::futures03::future::LocalBoxFuture<
                'a,
                flock::Result<flock::mssql_client::Transaction>,
            >
            {
                Box::pin(async move {
                    #standard
                    #translated
                    Ok(trans)
                })
            }
        }

        #test_merge_sql_schema
    })
}

enum MergeField<'a> {
    Field(&'a Field),
    Culture,
}

impl<'a> MergeField<'a> {
    fn is_identity(&self) -> bool {
        match self {
            Self::Field(f) => f.is_identity(),
            Self::Culture => false,
        }
    }
    fn is_key(&self) -> bool {
        match self {
            Self::Field(f) => f.is_key(),
            Self::Culture => true,
        }
    }

    fn name(&self) -> Result<String, TokenStream> {
        match self {
            Self::Field(f) => f.column(),
            Self::Culture => Ok("Culture".to_string()),
        }
    }
}

fn merge_execute<'a>(
    input: &DeriveInput,
    fields: impl Iterator<Item = MergeField<'a>>,
    table: &str,
    reload: bool,
) -> Result<TokenStream, TokenStream> {
    let mut errors = Vec::new();
    let mut insert_names = String::new();
    let mut insert_values = String::new();
    let mut params = Vec::new();
    let mut target_cond = String::new();
    let mut update_cond = String::new();
    let mut update_setters = String::new();
    let mut reload_keys = String::new();
    let mut reload_names = String::new();
    let mut reload_tokens = Vec::new();

    for (index, field) in fields.enumerate().map(|(i, f)| (i + 1, f)) {
        match field.name() {
            Ok(c) => {
                let is_identity = field.is_identity();
                let is_key = field.is_key();

                if is_key {
                    target_cond
                        .add_sep(" AND ")
                        .add_field_with_alias(&c, "t")
                        .add("=")
                        .add_param(index);
                } else if !is_identity {
                    update_setters
                        .add_sep(",")
                        .add_field(&c)
                        .add("=")
                        .add_param(index);

                    update_cond.add_sep(" OR ").add_diff(&c, index);
                }

                if reload {
                    if is_key {
                        reload_keys.add_sep(" AND ").add_field_with_alias(&c, "t");

                        if is_identity {
                            reload_keys.add("=Scope_Identity()");
                        } else {
                            reload_keys.add("=").add_param(index);
                        }
                    }

                    reload_names.add_sep(",").add_field_with_alias(&c, "t");

                    if let MergeField::Field(f) = field {
                        let i = f.ident()?;
                        let idx = LitInt::new(&reload_tokens.len().to_string(), f.span());

                        if f.is_translated() {
                            panic!("reloading translated values are not supported.");
                        } else {
                            reload_tokens.push(quote! { self.#i = row.get(#idx)?; })
                        }
                    }
                }

                if !is_identity {
                    insert_names.add_sep(",").add_field(&c);
                    insert_values.add_sep(",").add_param(index);
                }

                if reload {
                    match field {
                        MergeField::Field(f) => match f.ident() {
                            Ok(n) => params.push(if f.is_translated() {
                                quote! { self.#n[culture].to_string() }
                            } else {
                                quote! { self.#n.clone() }
                            }),
                            Err(e) => errors.push(e),
                        },
                        MergeField::Culture => params.push(quote! { culture }),
                    }
                } else {
                    match field {
                        MergeField::Field(f) => match f.ident() {
                            Ok(n) => params.push(if f.is_translated() {
                                quote! { &self.#n[culture] }
                            } else {
                                quote! { &self.#n }
                            }),
                            Err(e) => errors.push(e),
                        },
                        MergeField::Culture => params.push(quote! { culture }),
                    }
                }
            }
            Err(e) => errors.push(e),
        }
    }

    errors.result()?;

    if target_cond.is_empty() {
        return Err(Error::new(input.span(), "[key] attribute expected.").to_compile_error());
    }

    let update = if update_setters.is_empty() {
        String::new()
    } else {
        if !update_cond.is_empty() {
            update_cond.insert_str(0, "AND (");
            update_cond.push(')');
        }

        format!(
            "WHEN MATCHED {} THEN UPDATE SET {}",
            update_cond, update_setters
        )
    };

    let sql = if reload {
        format!(
            "SELECT {reload_names} FROM {table} t WHERE {reload_keys}",
            reload_keys = reload_keys,
            reload_names = reload_names,
            table = table,
        )
    } else {
        String::new()
    };

    let sql = format!(
        "MERGE {table} WITH (UPDLOCK) AS t USING (SELECT 1 A) AS s \
        ON {target_cond} \
        {update} \
        WHEN NOT MATCHED BY TARGET THEN INSERT ({insert_names}) VALUES ({insert_values});
        {reload}",
        insert_names = insert_names,
        insert_values = insert_values,
        table = table,
        target_cond = target_cond,
        update = update,
        reload = sql,
    );

    let sql = LitStr::new(&sql, input.span());

    let params = params.chunks(4).fold(quote! {()}, |q, v| {
        quote! { (#q, #(#v,)*) }
    });

    Ok(if reload {
        quote! {
            query_fold(#sql, #params, (), |_, row| {
                #(#reload_tokens)*
                Ok(())
            }).await?.0
        }
    } else {
        quote! { execute(#sql, #params).await? }
    })
}

fn test_merge_sql_schema(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    let fields = input.fields()?;
    let mut errors = Vec::new();
    let ident = &input.ident;
    let table_lit = input.table_lit().map_err(|e| errors.push(e));

    let mut fn_ident = ident.to_string().to_snake_case();
    fn_ident.insert_str(0, "test_merge_sql_schema_");

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

        let is_identity = LitBool {
            value: f.is_identity(),
            span: ident.span(),
        };

        Some(quote! {
            (
                #name,
                &<<#ty as FromColumn>::Value as SqlValue>::check_db_ty,
                Some(<<#ty as FromColumn>::Value as SqlValue>::is_nullable()),
                #is_key,
                #is_identity
            )
        })
    });

    let fields = quote! { #(#fields,)* };

    errors.result()?;

    let table = table_lit.expect("table");

    Ok(quote! {
        #[cfg(test)]
        #[tokio::test]
        async fn #fn_ident() {
            use flock::mssql_client::{FromColumn, SqlValue};
            flock::tests::test_merge_sql_schema(#table, &[#fields]).await;
        }
    })
}
