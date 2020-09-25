use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Ident, LitStr};

pub fn generate(input: DeriveInput) -> TokenStream {
    let ident = &input.ident;

    let mut set = ident.to_string().to_screaming_snake_case();
    set.push_str("_SET");

    let set = Ident::new(&set, ident.span());
    let ident_lit = LitStr::new(&ident.to_string(), ident.span());

    quote! {
        impl Copy for #ident {}

        impl std::clone::Clone for #ident {
            #[inline]
            fn clone(&self) -> Self {
                Self(self.0)
            }
        }

        impl std::fmt::Debug for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.debug_tuple(#ident_lit).field(&self.uuid()).finish()
            }
        }

        impl<'de> flock::serde::Deserialize<'de> for #ident {
            #[inline]
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: flock::serde::Deserializer<'de>,
            {
                Ok(Self::from(flock::Uuid::deserialize(deserializer)?))
            }
        }

        impl std::fmt::Display for #ident {
            #[inline]
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                self.uuid().fmt(f)
            }
        }

        impl std::cmp::Eq for #ident {}

        impl std::convert::From<#ident> for usize {
            #[inline]
            fn from(id: #ident) -> Self {
                id.0 as usize
            }
        }

        impl std::convert::From<usize> for #ident {
            #[inline]
            fn from(id: usize) -> Self {
                Self(id as _)
            }
        }

        impl std::convert::From<#ident> for flock::Uuid {
            #[inline]
            fn from(id: #ident) -> Self {
                id.uuid()
            }
        }

        impl std::convert::From<flock::Uuid> for #ident {
            #[inline]
            fn from(id: flock::Uuid) -> Self {
                Self(#set.get_or_create_index(id) as _)
            }
        }

        impl<'a> flock::mssql_client::FromColumn<'a> for #ident {
            type Value = flock::Uuid;
            #[inline]
            fn from_column(id: flock::Uuid) -> flock::mssql_client::Result<Self> {
                Ok(Self(#set.get_or_create_index(id) as _))
            }
        }

        impl std::hash::Hash for #ident {
            #[inline]
            fn hash<H>(&self, state: &mut H)
            where
                H: std::hash::Hasher,
            {
                self.0.hash(state)
            }
        }

        impl std::cmp::Ord for #ident {
            #[inline]
            fn cmp(&self, rhs: &Self) -> std::cmp::Ordering {
                self.0.cmp(&rhs.0)
            }
        }

        impl<'a> flock::mssql_client::Params<'a> for #ident {
            #[inline]
            fn params(self, out: &mut std::vec::Vec<flock::mssql_client::Parameter<'a>>) {
                flock::mssql_client::Params::params(self.uuid(), out);
            }

            #[inline]
            fn params_null(out: &mut std::vec::Vec<flock::mssql_client::Parameter<'a>>) {
                out.push(flock::mssql_client::Parameter::Uuid(None));
            }
        }

        impl std::cmp::PartialEq for #ident {
            #[inline]
            fn eq(&self, rhs: &Self) -> bool {
                self.0 == rhs.0
            }
        }

        impl std::cmp::PartialEq<flock::Uuid> for #ident {
            #[inline]
            fn eq(&self, rhs: &flock::Uuid) -> bool {
                #set.get_index(rhs).map_or(false, |i| i == (self.0 as usize))
            }
        }

        impl std::cmp::PartialOrd for #ident {
            #[inline]
            fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
                self.0.partial_cmp(&rhs.0)
            }
        }

        impl flock::serde::Serialize for #ident {
            #[inline]
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: flock::serde::Serializer,
            {
                self.uuid().serialize(serializer)
            }
        }

        impl flock::TryEntityIdFromUuid for #ident {
            #[inline]
            fn try_entity_id_from_uuid(u: flock::Uuid) -> Option<Self> {
                Self::try_get_from_uuid(u)
            }
        }

        impl #ident {
            #[inline]
            pub fn capacity() -> usize {
                #set.capacity()
            }

            #[inline]
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
