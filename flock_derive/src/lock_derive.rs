use either::Either;
use inflector::Inflector;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::path::PathBuf;
use std::{
    collections::hash_map::{Entry, HashMap},
    ops::Deref,
};
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{bracketed, parse_macro_input, parse_str, Error, Ident, LitStr, Token, Type};

type Config = HashMap<String, Vec<Type>>;

pub fn locks(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let args = parse_macro_input!(item as Args);
    derive(args).into()
}

pub fn locks_await(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let fields = parse_macro_input!(item as Fields);

    let args = Args {
        name: Ident::new("Locks", fields.span),
        fields,
    };

    let t = derive(args);

    quote!(
        #t
        let locks = Locks::lock().await?;
    )
    .into()
}

fn derive(args: Args) -> TokenStream {
    let as_muts = impl_as_muts(&args);
    let as_refs = impl_as_refs(&args);
    let locks = impl_struct(&args);
    let impl_tag_method = impl_tag_method(&args);
    let impl_lock_method = impl_lock_method(&args);

    let include_name = format!(
        "_INCLUDE_{}",
        args.name.to_string().to_screaming_snake_case()
    );
    let include_name = Ident::new(&include_name, args.name.span());
    let dir = config_dir();

    let dir = if let Some(dir) = dir {
        let dir = LitStr::new(
            &format!("{}", dir.display()),
            args.fields
                .iter()
                .map(|f| f.member.span())
                .next()
                .expect("field"),
        );

        quote! { const #include_name: &'static [u8] = include_bytes!(#dir); }
    } else {
        quote! {}
    };

    quote!(
        #locks
        #impl_lock_method
        #impl_tag_method
        #dir
        #as_muts
        #as_refs
    )
}

fn config_dir() -> Option<PathBuf> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?.replace("\\", "/");
    let mut p = PathBuf::from(manifest_dir);

    p.set_file_name("lock_derive.toml");

    if p.exists() {
        Some(p)
    } else {
        None
    }
}

fn load_config() -> Config {
    let p = match config_dir() {
        Some(p) => p,
        None => return Config::new(),
    };

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
    fn is_opt(self) -> bool {
        match self {
            Self::Read | Self::Write => false,
            Self::ReadOpt | Self::WriteOpt => true,
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
    fields: Fields,
    name: Ident,
}

impl Parse for Args {
    fn parse(stream: ParseStream) -> Result<Self> {
        let name = stream.parse()?;
        let _: Token![,] = stream.parse()?;

        Ok(Self {
            name,
            fields: stream.parse()?,
        })
    }
}

struct Fields {
    fields: Vec<Field>,
    span: Span,
}

impl Deref for Fields {
    type Target = Vec<Field>;

    fn deref(&self) -> &Self::Target {
        &self.fields
    }
}

impl Parse for Fields {
    fn parse(stream: ParseStream) -> Result<Self> {
        let mut set = HashMap::new();
        let config = load_config();
        let span = stream.span();

        while !stream.is_empty() {
            if !set.is_empty() {
                let _: Token![,] = stream.parse()?;
            }

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

        Ok(Self { fields, span })
    }
}

fn ty_to_string(t: &Type) -> String {
    quote!(#t).to_string()
}

struct Field {
    access: Access,
    member: Ident,
    ty: Type,
}

fn impl_lock_method(args: &Args) -> TokenStream {
    let resolve_guards = args.fields.iter().map(|f| {
        //let t = &f.ty;
        let n = &f.member;

        match f.access {
            Access::Read => {
                quote! {
                    let (conn, #n) = match flock::lock_read(conn).await {
                        Ok(v) => v,
                        Err(e) => return Err(e),
                    };
                }
            }
            Access::ReadOpt => {
                quote! { let #n = flock::lock_read_opt().await; }
            }
            Access::Write => {
                quote! {
                    let (conn, #n) = match flock::lock_write(conn).await {
                        Ok(v) => v,
                        Err(e) => return Err(e),
                    };
                }
            }
            Access::WriteOpt => {
                quote! { let #n = flock::lock_write_opt().await; }
            }
        }
    });

    let fields = args.fields.iter().map(|f| {
        let n = &f.member;
        quote! { #n: #n }
    });

    let name = &args.name;

    quote! {
        impl #name {
            pub async fn lock() -> flock::Result<Self> {
                let conn = flock::ConnOrFactory::from_env("DB")?;

                #(#resolve_guards)*

                Ok(Self {
                    #(#fields,)*
                })
            }
        }

        impl flock::DoLock for #name {
            fn do_lock() -> flock::futures03::future::LocalBoxFuture<'static, flock::Result<Self>> {
                Box::pin(Self::lock())
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

    let name = &args.name;

    quote! {
        struct #name {
            #(#fields,)*
        }
    }
}

fn impl_tag_method(args: &Args) -> TokenStream {
    let name = &args.name;

    let fields = args.fields.iter().map(|f| {
        let n = &f.member;

        match f.access {
            Access::Read => quote! { self.#n.tag() },
            Access::ReadOpt => quote! { (&*self.#n).as_ref()?.tag()  },
            Access::Write => quote! { self.#n.tag() },
            Access::WriteOpt => quote! { (&*self.#n).as_ref()?.tag() },
        }
    });

    if args.fields.iter().any(|f| f.access.is_opt()) {
        quote! {
            // impl #name {
            //     pub fn tag(&self) -> Option<flock::version_tag::VersionTag> {
            //         Some(flock::version_tag::combine(&[#(#fields,)*]))
            //     }
            // }
        }
    } else {
        quote! {
            impl #name {
                pub fn tag(&self) -> flock::version_tag::VersionTag {
                    flock::version_tag::combine(&[#(#fields,)*])
                }
            }

            impl From<&#name> for flock::version_tag::VersionTag {
                fn from(l: &#name) -> Self {
                    l.tag()
                }
            }
        }
    }
}

fn impl_as_muts(args: &Args) -> TokenStream {
    let name = &args.name;

    let fields = args.fields.iter().filter_map(|f| {
        let t = &f.ty;
        let n = &f.member;

        match f.access {
            Access::Read | Access::ReadOpt => None,
            Access::Write => Some(quote! {
                impl AsMut<#t> for #name {
                    fn as_mut(&mut self) -> &mut #t {
                        &mut self.#n
                    }
                }
            }),
            Access::WriteOpt => Some(quote! {
                impl AsMut<Option<#t>> for #name {
                    fn as_mut(&mut self) -> &mut Option<#t> {
                        &mut self.#n
                    }
                }

                impl flock::AsMutOpt<#t> for #name {
                    fn as_mut_opt(&mut self) -> Option<&mut #t> {
                        self.#n.as_mut_opt()
                    }
                }
            }),
        }
    });

    quote! { #(#fields)* }
}

fn impl_as_refs(args: &Args) -> TokenStream {
    let name = &args.name;

    let fields = args.fields.iter().map(|f| {
        let t = &f.ty;
        let n = &f.member;

        match f.access {
            Access::Read | Access::Write => quote! {
                impl AsRef<#t> for #name {
                    fn as_ref(&self) -> &#t {
                        &self.#n
                    }
                }
            },
            Access::ReadOpt | Access::WriteOpt => quote! {
                impl AsRef<Option<#t>> for #name {
                    fn as_ref(&self) -> &Option<#t> {
                        &self.#n
                    }
                }
            },
        }
    });

    quote! { #(#fields)* }
}
