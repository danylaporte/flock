use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields, Ident};

pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let as_mut = impl_as_mut(&input);
    let as_ref = impl_as_ref(&input);
    let deref = impl_deref(&input);
    let load_test = impl_load_test(&input);
    let lock = impl_lock(&input);
    let set_tag = impl_set_tag(&input);
    let tag = impl_tag(&input);

    quote! (
        #as_mut
        #as_ref
        #deref
        #load_test
        #lock
        #set_tag
        #tag
    )
    .into()
}

fn get_field_by_name<'a>(input: &'a DeriveInput, name: &str) -> Option<&'a Field> {
    if let Data::Struct(d) = &input.data {
        if let Fields::Named(n) = &d.fields {
            return n.named.iter().find(|f| {
                f.ident
                    .as_ref()
                    .map(|f| f.to_string() == name)
                    .unwrap_or(false)
            });
        }
    }

    None
}

fn impl_deref(input: &DeriveInput) -> TokenStream {
    if let Some(entities_field) = get_field_by_name(input, "entities") {
        let field_ty = &entities_field.ty;
        let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
        let t = &input.ident;

        quote! {
            impl #impl_generics std::ops::Deref for #t #ty_generics #where_clause {
                type Target = #field_ty;

                fn deref(&self) -> &Self::Target {
                    &self.entities
                }
            }

            impl #impl_generics std::ops::DerefMut for #t #ty_generics #where_clause {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.entities
                }
            }
        }
    } else {
        quote! {}
    }
}

fn impl_as_mut(input: &DeriveInput) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let t = &input.ident;

    quote! {
        impl #impl_generics AsMut<#t #ty_generics> for #t #ty_generics #where_clause {
            fn as_mut(&mut self) -> &mut Self {
                self
            }
        }
    }
}

fn impl_as_ref(input: &DeriveInput) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let t = &input.ident;

    quote! {
        impl #impl_generics AsRef<#t #ty_generics> for #t #ty_generics #where_clause {
            fn as_ref(&self) -> &Self {
                self
            }
        }
    }
}

fn impl_load_test(input: &DeriveInput) -> TokenStream {
    if input.generics.params.is_empty() {
        let t = &input.ident;

        quote! {
            #[test]
            fn test_load() {
                use flock::tokio::executor::current_thread::block_on_all;
                use flock::LoadFromConn;

                let fut = flock::mssql_client::Connection::from_env("DB").and_then(#t::load_from_conn);
                block_on_all(fut).unwrap();
            }
        }
    } else {
        quote! {}
    }
}

fn impl_set_tag(input: &DeriveInput) -> TokenStream {
    let t = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics flock::SetTag for #t #ty_generics #where_clause {
            fn set_tag(&mut self, tag: flock::version_tag::VersionTag) {
                self.tag = tag;
            }
        }
    }
}

fn impl_tag(input: &DeriveInput) -> TokenStream {
    let t = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics #t #ty_generics #where_clause {
            pub fn tag(&self) -> flock::version_tag::VersionTag {
                self.tag
            }
        }
    }
}

fn impl_lock(input: &DeriveInput) -> TokenStream {
    if input.generics.params.is_empty() {
        let ident = &input.ident;
        let lock = format!("{}_LOCK", &ident).to_screaming_snake_case();
        let lock = Ident::new(&lock, ident.span());

        quote! {
            static #lock: flock::Lock<#ident> = flock::Lock::new();

            impl flock::AsLock for #ident {
                fn as_lock() -> &'static flock::Lock<Self> {
                    &#lock
                }
            }
        }
    } else {
        quote! {}
    }
}
