use super::{DeriveInputExt, Errors, FieldExt, SqlStringExt};
use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, DeriveInput, Field, Ident, LitStr};

pub(crate) fn delete(input: &DeriveInput) -> TokenStream {
    try_token_stream!(impl_delete(input))
}

fn impl_delete(input: &DeriveInput) -> Result<TokenStream, TokenStream> {
    let ident = &input.ident;
    let fields = input.fields()?;

    let fields = fields.iter().filter(|f| f.is_key()).collect::<Vec<_>>();
    let execute = delete_execute(&input, &fields, &input.table()?)?;
    let standard = quote! { trans = trans.#execute; };

    let translated = if fields.iter().any(|f| f.is_translated()) {
        let table = input.translated_attr()?.table;
        let execute = delete_execute(&input, &fields, &table.value())?;
        quote! { trans = trans.#execute; }
    } else {
        quote! {}
    };

    let test_delete_sql_schema = test_delete_sql_schema(input, &fields)?;

    Ok(quote! {
        impl flock::DeleteSql for #ident {
            fn delete_sql<'a>(
                &'a self,
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

        #test_delete_sql_schema
    })
}

fn delete_execute(
    input: &DeriveInput,
    keys: &Vec<&Field>,
    table: &str,
) -> Result<TokenStream, TokenStream> {
    let mut errors = Vec::new();
    let mut params = Vec::new();
    let mut where_clauses = String::new();

    for (index, field) in keys.iter().enumerate().map(|(i, f)| (i + 1, f)) {
        match field.column() {
            Ok(c) => {
                let field = match field.ident() {
                    Ok(field) => field,
                    Err(e) => {
                        errors.push(e);
                        continue;
                    }
                };

                where_clauses
                    .add_sep(" AND ")
                    .add_field(&c)
                    .add("=")
                    .add_param(index);

                params.push(quote! { &self.#field });
            }
            Err(e) => errors.push(e),
        }
    }

    errors.result()?;

    let sql = format!(
        "DELETE FROM {table} WHERE {where_clauses}",
        table = table,
        where_clauses = where_clauses
    );

    let sql = LitStr::new(&sql, input.span());

    let params = params.chunks(4).fold(quote! {()}, |q, v| {
        quote! { (#q, #(#v,)*) }
    });

    Ok(quote! { execute(#sql, #params).await? })
}

fn test_delete_sql_schema(
    input: &DeriveInput,
    fields: &Vec<&Field>,
) -> Result<TokenStream, TokenStream> {
    let mut errors = Vec::new();
    let ident = &input.ident;
    let table_lit = input.table_lit().map_err(|e| errors.push(e));

    let mut fn_ident = ident.to_string().to_snake_case();
    fn_ident.insert_str(0, "test_delete_sql_schema_");

    let fn_ident = Ident::new(&fn_ident, input.span());

    let fields = fields.iter().filter_map(|f| {
        if f.is_translated() {
            return None;
        }

        let column = f.column().map_err(|e| errors.push(e)).ok()?;
        let name = LitStr::new(&column.to_lowercase(), f.span());
        let ty = f.ty();

        Some(quote! {
            (
                #name,
                &<<#ty as FromColumn>::Value as SqlValue>::check_db_ty,
                Some(<<#ty as FromColumn>::Value as SqlValue>::is_nullable()),
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
            flock::tests::test_delete_sql_schema(#table, &[#fields]).await;
        }
    })
}
