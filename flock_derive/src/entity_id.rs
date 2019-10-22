use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, LitStr};

pub fn generate(input: DeriveInput) -> TokenStream {
    let ident = &input.ident;

    let set = Ident::new(
        &format!("{}_SET", ident).to_screaming_snake_case(),
        ident.span(),
    );

    let ident_lit = LitStr::new(&ident.to_string(), ident.span());

    quote! {
        impl Copy for #ident {}

        impl Clone for #ident {
            fn clone(&self) -> Self {
                Self(self.0)
            }
        }

        impl std::fmt::Debug for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.debug_tuple(#ident_lit).field(&self.uuid()).finish()
            }
        }

        impl std::fmt::Display for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                self.uuid().fmt(f)
            }
        }

        impl<'de> flock::serde::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: flock::serde::Deserializer<'de>,
            {
                Ok(Self::from(flock::Uuid::deserialize(deserializer)?))
            }
        }

        impl Eq for #ident {}

        impl From<#ident> for usize {
            fn from(id: #ident) -> Self {
                id.0 as usize
            }
        }

        impl From<usize> for #ident {
            fn from(id: usize) -> Self {
                Self(id as _)
            }
        }

        impl From<#ident> for flock::Uuid {
            fn from(id: #ident) -> Self {
                id.uuid()
            }
        }

        impl From<flock::Uuid> for #ident {
            fn from(id: flock::Uuid) -> Self {
                Self(#set.get_or_create_index(id) as _)
            }
        }

        impl<'a> flock::mssql_client::FromColumn<'a> for #ident {
            type Value = flock::Uuid;
            fn from_column(id: flock::Uuid) -> Result<Self, flock::failure::Error> {
                Ok(Self(#set.get_or_create_index(id) as _))
            }
        }

        impl std::hash::Hash for #ident {
            fn hash<H>(&self, state: &mut H)
            where
                H: std::hash::Hasher,
            {
                self.0.hash(state)
            }
        }

        impl Ord for #ident {
            fn cmp(&self, rhs: &Self) -> std::cmp::Ordering {
                self.0.cmp(&rhs.0)
            }
        }

        impl<'a> flock::mssql_client::Params<'a> for #ident {
            fn params(self, out: &mut Vec<flock::mssql_client::Parameter<'a>>) {
                let id = self.uuid();
                flock::mssql_client::Params::params(id, out);
            }

            fn params_null(out: &mut Vec<flock::mssql_client::Parameter<'a>>) {
                out.push(flock::mssql_client::Parameter::Uuid(None));
            }
        }

        impl PartialEq for #ident {
            fn eq(&self, rhs: &Self) -> bool {
                self.0 == rhs.0
            }
        }

        impl PartialEq<flock::Uuid> for #ident {
            fn eq(&self, rhs: &flock::Uuid) -> bool {
                #set.get_index(rhs).map_or(false, |i| i == (self.0 as usize))
            }
        }

        impl PartialOrd for #ident {
            fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
                self.0.partial_cmp(&rhs.0)
            }
        }

        impl flock::serde::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: flock::serde::Serializer,
            {
                self.uuid().serialize(serializer)
            }
        }

        impl #ident {
            pub fn capacity() -> usize {
                #set.capacity()
            }

            pub fn try_get_from_uuid(id: flock::Uuid) -> Option<#ident> {
                Some(#ident(#set.get_index(&id)? as _))
            }

            fn uuid(&self) -> flock::Uuid {
                #set.get_uuid(self.0 as usize).expect("Uuid")
            }
        }

        static #set: flock::EntityIdSet = flock::EntityIdSet::new();
    }
}
