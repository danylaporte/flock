use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields};

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
        let generics = &input.generics;
        let t = &input.ident;

        quote! {
            impl #generics std::ops::Deref for #t #generics {
                type Target = #field_ty;

                fn deref(&self) -> &Self::Target {
                    &self.entities
                }
            }

            impl #generics std::ops::DerefMut for #t #generics {
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
    let generics = &input.generics;
    let t = &input.ident;

    quote! {
        impl #generics AsMut<#t> for #t {
            fn as_mut(&mut self) -> &mut Self {
                self
            }
        }
    }
}

fn impl_as_ref(input: &DeriveInput) -> TokenStream {
    let generics = &input.generics;
    let t = &input.ident;

    quote! {
        impl #generics AsRef<#t> for #t {
            fn as_ref(&self) -> &Self {
                self
            }
        }
    }
}

fn impl_load_test(input: &DeriveInput) -> TokenStream {
    let t = &input.ident;

    quote! {
        #[test]
        fn test_load() {
            use tokio::executor::current_thread::block_on_all;
            use flock::LoadFromConn;

            let fut = mssql_client::Connection::from_env("COT_DB").and_then(#t::load_from_conn);
            block_on_all(fut).unwrap();
        }
    }
}

fn impl_set_tag(input: &DeriveInput) -> TokenStream {
    let t = &input.ident;
    let generics = &input.generics;

    quote! {
        impl #generics flock::SetTag for #t {
            fn set_tag(&mut self, tag: flock::VersionTag) {
                self.tag = tag;
            }
        }
    }
}

fn impl_tag(input: &DeriveInput) -> TokenStream {
    let t = &input.ident;
    let generics = &input.generics;

    quote! {
        impl #generics #t #generics {
            pub fn tag(&self) -> flock::VersionTag {
                self.tag
            }
        }
    }
}

fn impl_lock(input: &DeriveInput) -> TokenStream {
    let generics = &input.generics;
    let t = &input.ident;

    quote! {
        impl #generics flock::AsLock for #t #generics {
            type Lock = &'static flock::Lock<Self>;

            fn as_lock() -> Self::Lock {
                flock::lazy_static::lazy_static! {
                    static ref LOCK: flock::Lock<#t> = flock::Lock::default();
                }

                &*LOCK
            }
        }
    }
}
