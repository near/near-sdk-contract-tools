use darling::{FromDeriveInput, FromVariant};
use proc_macro2::TokenStream;
use quote::quote;

use crate::rename::RenameStrategy;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep297), supports(struct_any))]
pub struct Nep297Meta {
    pub standard: String,
    pub version: String,
    pub name: Option<String>,
    pub rename: Option<RenameStrategy>,
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
        name,
        rename,
        ident: type_name,
        generics,
        data,
        me,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    // Variant attributes
    let event = match &data {
        darling::ast::Data::Struct(_) => {
            let transformed_name = if let Some(name) = name {
                name
            } else if let Some(rename) = rename {
                rename.transform(&type_name.to_string())
            } else {
                type_name.to_string()
            };

            quote! { #transformed_name }
        }
        _ => unreachable!(),
    };

    Ok(quote! {
        impl #imp #me::standard::nep297::ToEventLog for #type_name #ty #wher {
            type Data = #type_name #ty;

            fn to_event_log<'geld>(&'geld self) -> #me::standard::nep297::EventLog<&'geld Self> {
                #me::standard::nep297::EventLog {
                    standard: #standard,
                    version: #version,
                    event: #event,
                    data: self,
                }
            }
        }
    })
}
