use either::Either;
use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use std::collections::hash_map::{Entry, HashMap};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{bracketed, parse_macro_input, parse_str, Error, Ident, Token, Type};

type Config = HashMap<String, Vec<Type>>;

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

    let config: HashMap<String, Vec<String>> =
        toml::from_str(&s).expect("deserialize lock_derive.toml");

    let mut out = HashMap::with_capacity(config.len());

    for (key, value) in config {
        out.insert(
            key,
            value
                .into_iter()
                .map(|v| parse_str(&v).expect("unparsable type"))
                .collect(),
        );
    }

    out
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

fn ty_to_string(t: &Type) -> String {
    quote!(#t).to_string()
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

            let punctuated = <Punctuated<Type, Token![,]>>::parse_terminated(&content)?;
            let vec = punctuated.into_iter().collect::<Vec<_>>();

            for ty in vec {
                let s = ty_to_string(&ty);

                let names = config
                    .get(&s)
                    .as_ref()
                    .map(|v| Either::Left(v.iter()))
                    .unwrap_or_else(|| Either::Right(std::iter::once(&ty)));

                for name in names {
                    let s = ty_to_string(&name);
                    let key = s.split("::").last().expect("key").trim().to_string();
                    let member = Ident::new(&key.to_snake_case(), ty.span());

                    match set.entry(key) {
                        Entry::Vacant(v) => {
                            let ty = name.clone();
                            v.insert(Field { access, member, ty });
                        }
                        Entry::Occupied(mut o) => {
                            o.get_mut().access += access;
                        }
                    }
                }
            }
        }

        let mut fields = set.into_iter().map(|(_, v)| v).collect::<Vec<_>>();
        fields.sort_unstable_by(|a, b| a.member.cmp(&b.member));

        Ok(Self { fields })
    }
}

struct Field {
    access: Access,
    member: Ident,
    ty: Type,
}

fn impl_resolve(args: &Args) -> TokenStream {
    let fields = args.fields.iter().map(|f| &f.member);
    let mut inner_code = Some(quote! { Ok(Locks { #(#fields,)* }) });

    for f in args.fields.iter() {
        let t = &f.ty;

        let access = match f.access {
            Access::Read => quote! { read(conn) },
            Access::ReadOpt => quote! { read_opt() },
            Access::Write => quote! { write(conn) },
            Access::WriteOpt => quote! { write_opt() },
        };

        let v = &f.member;
        let code = inner_code.take().expect("inner_code");

        inner_code = Some(quote! { #t::as_lock().#access.and_then(move|(conn, #v)| #code) });
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
        let t = &f.ty;
        let n = &f.member;

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
        let t = &f.ty;
        let n = &f.member;

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
        let t = &f.ty;
        let n = &f.member;

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
