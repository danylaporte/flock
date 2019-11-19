use inflector::Inflector;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    spanned::Spanned, Error, FnArg, GenericArgument, Ident, ItemTrait, Pat, PatType, PathArguments,
    ReturnType, TraitItem, TraitItemMethod, Type, TypePath,
};

pub fn generate(input: ItemTrait) -> TokenStream {
    let items = &input.items;
    let mut errors = Vec::new();

    let relations = items
        .iter()
        .filter_map(|item| match item {
            TraitItem::Method(m) => Some(relation(m)),
            _ => {
                errors.push(Error::new(item.span(), "Fn expected.").to_compile_error());
                None
            }
        })
        .collect::<Vec<_>>();

    if errors.is_empty() {
        let inits = relations.iter().map(|r| &r.init);
        let fields = relations.iter().map(|r| &r.field);
        let methods = relations.iter().map(|r| &r.method);
        let types = relations.iter().map(|r| &r.relation);

        quote! {
            #(#types)*

            pub struct Relations<L> {
                pub locks: L,
                #(#fields)*
            }

            impl<L> Relations<L> {
                #(#methods)*
            }

            impl<L> From<L> for Relations<L> {
                fn from(locks: L) -> Self {
                    Self {
                        locks,
                        #(#inits)*
                    }
                }
            }
        }
    } else {
        quote! { #(#errors)* }
    }
}

fn relation(f: &TraitItemMethod) -> RelationData {
    let ident = &f.sig.ident;
    let ident_str = ident.to_string();
    let relation = Ident::new(&ident_str.to_pascal_case(), ident.span());
    let static_ident = Ident::new(&ident_str.to_screaming_snake_case(), ident.span());
    let args = &f.sig.inputs;

    let wheres = args.iter().map(|a| match a.ty() {
        Ok(t) => quote! { AsRef<#t> + },
        Err(e) => e,
    });

    let wheres = quote! { #(#wheres)* };

    let as_refs = args
        .iter()
        .filter_map(|a| a.ident_and_ty().ok())
        .map(|(i, t)| quote! { let #i: &#t = locks.as_ref(); });

    let as_refs = quote! { #(#as_refs)* };

    let tags = args
        .iter()
        .filter_map(|a| a.ident().ok())
        .map(|i| quote! { #i.tag() });

    let tags = quote! { #(#tags),* };
    let mut extra_methods = quote! {};

    let relation_ty = match &f.sig.output {
        ReturnType::Type(_, t) => {
            extra_methods = relation_methods(t);
            quote! { #t }
        }
        _ => Error::new(f.sig.output.span(), "Output parameter required.").to_compile_error(),
    };

    let body = match &f.default {
        Some(block) => quote! { #block },
        None => Error::new(f.span(), "Expect a method body").to_compile_error(),
    };

    RelationData {
        field: quote! { #ident: flock::OnceCell<#relation>, },
        init: quote! { #ident: flock::OnceCell::new(), },
        method: quote! {
            pub fn #ident(&self) -> &#relation
            where
                L: #wheres
            {
                let locks = &self.locks;
                self.#ident.get_or_init(|| #relation::from(locks))
            }
        },
        relation: quote! {
            static #static_ident: flock::version_cache::VersionCache<#relation_ty> = flock::version_cache::VersionCache::new();

            pub struct #relation(flock::version_cache::CacheData<#relation_ty>);

            impl std::ops::Deref for #relation {
                type Target = #relation_ty;

                fn deref(&self) -> &Self::Target {
                    &*self.0
                }
            }

            impl<L> From<L> for #relation where L: #wheres {
                fn from(locks: L) -> Self {
                    #as_refs
                    Self(#static_ident.get_or_init(|| #body, &[#tags]))
                }
            }

            impl #relation {
                #extra_methods

                pub fn tag(&self) -> flock::version_tag::VersionTag {
                    (self.0).tag()
                }
            }
        },
    }
}

fn relation_methods(ty: &Type) -> TokenStream {
    if let Some((left, right)) = extract_2_generics_ty(ty, "ManyToMany") {
        let left_ids_by = Ident::new(
            &format!(
                "{}_by",
                quote! { #left }.to_string().to_snake_case().to_plural()
            ),
            left.span(),
        );
        let right_ids_by = Ident::new(
            &format!(
                "{}_by",
                quote! { #right }.to_string().to_snake_case().to_plural()
            ),
            right.span(),
        );

        quote! {
            pub fn #left_ids_by(&self, id: #right) -> flock::iter::ManyIter<#left> {
                (self.0).iter_left_by(id)
            }

            pub fn #right_ids_by(&self, id: #left) -> flock::iter::ManyIter<#right> {
                (self.0).iter_right_by(id)
            }
        }
    } else if let Some((one, many)) = extract_2_generics_ty(ty, "OneToMany") {
        let many_ids_by = Ident::new(
            &format!(
                "{}_by",
                quote! { #many }.to_string().to_snake_case().to_plural()
            ),
            many.span(),
        );

        quote! {
            pub fn #many_ids_by(&self, id: #one) -> flock::iter::ManyIter<#many> {
                (self.0).iter_by(id)
            }
        }
    } else {
        quote! {}
    }
}

struct RelationData {
    field: TokenStream,
    init: TokenStream,
    method: TokenStream,
    relation: TokenStream,
}

trait FnArgExt {
    fn as_arg(&self) -> &FnArg;

    fn ident(&self) -> Result<&Ident, TokenStream> {
        self.ident_and_ty().map(|t| t.0)
    }

    fn ident_and_ty(&self) -> Result<(&Ident, &Type), TokenStream> {
        match self.as_arg() {
            FnArg::Typed(PatType { pat, ty, .. }) => match pat.as_ref() {
                Pat::Ident(i) => Ok((&i.ident, ty)),
                v => Err(Error::new(v.span(), "Unsupported type parameter").to_compile_error()),
            },
            FnArg::Receiver(v) => {
                Err(Error::new(v.span(), "Unsupported type parameter").to_compile_error())
            }
        }
    }

    fn ty(&self) -> Result<&Type, TokenStream> {
        self.ident_and_ty().map(|t| t.1)
    }
}

impl FnArgExt for FnArg {
    fn as_arg(&self) -> &FnArg {
        self
    }
}

fn extract_2_generics_ty<'a>(t: &'a Type, check_name: &str) -> Option<(&'a Type, &'a Type)> {
    if let Type::Path(TypePath { path, .. }) = t {
        let last = path
            .segments
            .iter()
            .last()
            .filter(|v| v.ident == check_name)
            .map(|v| &v.arguments);

        if let Some(PathArguments::AngleBracketed(b)) = last {
            let mut iter = b.args.iter().filter_map(|a| match a {
                GenericArgument::Type(t) => Some(t),
                _ => None,
            });
            return Some((iter.next()?, iter.next()?));
        }
    }

    None
}
