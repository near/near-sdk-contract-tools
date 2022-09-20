use darling::{ast::Style, FromDeriveInput, FromVariant};
use proc_macro2::TokenStream;
use quote::quote;

use crate::rename::RenameStrategy;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep297), supports(enum_any))]
pub struct Nep297Meta {
    pub standard: String,
    pub version: String,
    pub rename_all: Option<RenameStrategy>,

    pub ident: syn::Ident,
    pub generics: syn::Generics,
    pub data: darling::ast::Data<EventVariantReceiver, ()>,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
}

#[derive(Debug, FromVariant)]
#[darling(attributes(nep297))]
pub struct EventVariantReceiver {
    pub ident: syn::Ident,
    pub fields: darling::ast::Fields<()>,
    pub rename: Option<RenameStrategy>,
    pub name: Option<String>,
}

pub fn expand(meta: Nep297Meta) -> Result<TokenStream, darling::Error> {
    let Nep297Meta {
        standard,
        version,
        rename_all,
        ident: type_name,
        generics,
        data,
        me,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    // Variant attributes
    let arms = match &data {
        darling::ast::Data::Enum(variants) => variants,
        _ => unreachable!(), // Because of darling supports(enum_any) above
    }
    .iter()
    .map(
        |EventVariantReceiver {
             ident,
             fields,
             rename,
             name,
         }| {
            let transformed_name = if let Some(name) = name {
                name.to_string()
            } else if let Some(rename) = rename {
                rename.transform(&ident.to_string())
            } else if let Some(rename_all) = &rename_all {
                rename_all.transform(&ident.to_string())
            } else {
                ident.to_string()
            };

            match fields.style {
                Style::Unit => quote! { #type_name :: #ident => #transformed_name , },
                Style::Tuple => {
                    quote! { #type_name :: #ident (..) => #transformed_name , }
                }
                Style::Struct => {
                    quote! { #type_name :: #ident {..} => #transformed_name , }
                }
            }
        },
    )
    .collect::<Vec<_>>();

    Ok(quote! {
        impl #imp #me::standard::nep297::EventMetadata for #type_name #ty #wher {
            fn standard(&self) -> &'static str {
                #standard
            }

            fn version(&self) -> &'static str {
                #version
            }

            fn event(&self) -> &'static str {
                match self {
                    #(#arms)*
                }
            }
        }

        impl #imp ::std::fmt::Display for #type_name #ty #wher {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(
                    f,
                    "{}",
                    #me::standard::nep297::Event::to_event_string(self),
                )
            }
        }
    })
}
