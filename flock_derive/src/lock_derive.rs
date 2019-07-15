use either::Either;
use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::hash_map::{Entry, HashMap};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{bracketed, parse_macro_input, Error, Ident, Token};

type Config = HashMap<String, Vec<String>>;

pub fn derive(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(item as Args);
    let as_muts = impl_as_muts(&args);
    let as_refs = impl_as_refs(&args);
    let locks = impl_struct(&args);
    let resolve = impl_resolve(&args);

    quote!({
        #as_muts
        #as_refs
        #locks
        #resolve

        Locks::resolve()
    })
    .into()
}

fn load_config() -> Config {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR")
        .replace("\\", "/");

    let mut p = std::path::PathBuf::from(manifest_dir);
    p.set_file_name("lock_derive.toml");

    let s = std::fs::read_to_string(&p).unwrap_or_else(|e| {
        panic!("Failed to open `{}`; {}", p.display(), e);
    });

    toml::from_str(&s).expect("deserialize lock_derive.toml")
}

#[derive(Clone, Copy)]
enum Access {
    Read,
    ReadOpt,
    Write,
    WriteOpt,
}

impl Access {
    fn is_write(self) -> bool {
        match self {
            Access::Read | Access::ReadOpt => false,
            Access::Write | Access::WriteOpt => true,
        }
    }
}

impl std::ops::AddAssign for Access {
    fn add_assign(&mut self, rhs: Self) {
        let o = match (*self, rhs) {
            (Access::Write, _)
            | (_, Access::Write)
            | (Access::WriteOpt, Access::Read)
            | (Access::Read, Access::WriteOpt) => Access::Write,
            (Access::WriteOpt, Access::WriteOpt)
            | (Access::WriteOpt, Access::ReadOpt)
            | (Access::ReadOpt, Access::WriteOpt) => Access::WriteOpt,
            (Access::ReadOpt, Access::ReadOpt) => Access::ReadOpt,
            (Access::Read, Access::Read)
            | (Access::Read, Access::ReadOpt)
            | (Access::ReadOpt, Access::Read) => Access::Read,
        };

        *self = o;
    }
}

impl Parse for Access {
    fn parse(stream: ParseStream) -> Result<Self> {
        let name: Ident = stream.parse()?;
        match name.to_string().as_str() {
            "read" => Ok(Access::Read),
            "read_opt" => Ok(Access::ReadOpt),
            "write" => Ok(Access::Write),
            "write_opt" => Ok(Access::WriteOpt),
            _ => Err(Error::new(
                name.span(),
                "Expect `read`, `read_opt`, `write` or `write_opt`.",
            )),
        }
    }
}

struct Args {
    fields: Vec<Field>,
}

impl Parse for Args {
    fn parse(stream: ParseStream) -> Result<Self> {
        let mut set = HashMap::new();
        let config = load_config();

        while !stream.is_empty() {
            let access: Access = stream.parse()?;
            let _: Token![:] = stream.parse()?;

            let content;
            bracketed!(content in stream);

            let punctuated = <Punctuated<Ident, Token![,]>>::parse_terminated(&content)?;
            let vec = punctuated.into_iter().collect::<Vec<_>>();

            for ident in vec {
                let s = ident.to_string();

                let names = config
                    .get(&s)
                    .as_ref()
                    .map(|v| Either::Left(v.iter()))
                    .unwrap_or_else(|| Either::Right(std::iter::once(&s)));

                for name in names {
                    match set.entry(name.clone()) {
                        Entry::Vacant(v) => {
                            let ident = Ident::new(name, ident.span());
                            v.insert(Field { ident, access });
                        }
                        Entry::Occupied(mut o) => {
                            o.get_mut().access += access;
                        }
                    }
                }
            }
        }

        let mut fields = set.into_iter().map(|(_, v)| v).collect::<Vec<_>>();
        fields.sort_unstable_by(|a, b| a.ident.cmp(&b.ident));

        Ok(Self { fields })
    }
}

struct Field {
    access: Access,
    ident: Ident,
}

fn impl_resolve(args: &Args) -> TokenStream {
    let fields = args.fields.iter().map(|f| {
        let v = Ident::new(&f.ident.to_string().to_snake_case(), f.ident.span());
        quote! { #v }
    });

    let mut inner_code = Some(quote! { Ok(Locks { #(#fields,)* }) });

    for f in args.fields.iter() {
        let ident = &f.ident;

        let access = match f.access {
            Access::Read => quote! { read(conn) },
            Access::ReadOpt => quote! { read_opt() },
            Access::Write => quote! { write(conn) },
            Access::WriteOpt => quote! { write_opt() },
        };

        let v = Ident::new(&ident.to_string().to_snake_case(), ident.span());
        let code = inner_code.take().expect("inner_code");

        inner_code = Some(quote! { #ident::as_lock().#access.and_then(move|(conn, #v)| #code) });
    }

    let code = inner_code.expect("inner_code");

    quote! {
        impl Locks {
            fn resolve() -> impl futures::Future<Item = Self, Error = failure::Error> {
                use flock::{ConnOrFactory, ConnectionFactory, AsLock};
                use futures::Future;

                let conn = ConnOrFactory::Factory(ConnectionFactory::from_env("COT_DB").expect("COT_DB"));

                #code.map(|(_conn, locks)| locks)
            }
        }
    }
}

fn impl_struct(args: &Args) -> TokenStream {
    let fields = args.fields.iter().map(|f| {
        let t = &f.ident;
        let n = Ident::new(&t.to_string().to_snake_case(), f.ident.span());

        match f.access {
            Access::Read => quote! { #n: flock::ReadGuard<#t> },
            Access::ReadOpt => quote! { #n: flock::ReadOptGuard<#t> },
            Access::Write => quote! { #n: flock::WriteGuard<#t> },
            Access::WriteOpt => quote! { #n: flock::WriteOptGuard<#t> },
        }
    });

    quote! {
        struct Locks {
            #(#fields,)*
        }
    }
}

fn impl_as_muts(args: &Args) -> TokenStream {
    let fields = args.fields.iter().filter(|f| f.access.is_write()).map(|f| {
        let t = &f.ident;
        let n = Ident::new(&t.to_string().to_snake_case(), t.span());

        quote! {
            impl AsMut<#t> for Locks {
                fn as_mut(&mut self) -> &mut #t {
                    &mut *self.#n
                }
            }
        }
    });

    quote! { #(#fields)* }
}

fn impl_as_refs(args: &Args) -> TokenStream {
    let fields = args.fields.iter().map(|f| {
        let t = &f.ident;
        let n = Ident::new(&t.to_string().to_snake_case(), t.span());

        quote! {
            impl AsRef<#t> for Locks {
                fn as_ref(&self) -> &#t {
                    &*self.#n
                }
            }
        }
    });

    quote! { #(#fields)* }
}
