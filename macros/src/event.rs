use darling::{ast::Style, FromDeriveInput, FromVariant};
use proc_macro::TokenStream;
use quote::quote;

use crate::rename::RenameStrategy;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(event), supports(enum_any))]
pub struct EventMeta {
    pub standard: String,
    pub version: String,
    pub rename_all: Option<RenameStrategy>,

    pub ident: syn::Ident,
    pub data: darling::ast::Data<EventVariantReceiver, ()>,
}

#[derive(Debug, FromVariant)]
#[darling(attributes(event))]
pub struct EventVariantReceiver {
    pub ident: syn::Ident,
    pub fields: darling::ast::Fields<()>,
    pub rename: Option<RenameStrategy>,
    pub name: Option<String>,
}

pub fn expand(meta: EventMeta) -> Result<TokenStream, syn::Error> {
    let EventMeta {
        standard,
        version,
        rename_all,
        ident: type_name,
        data,
    } = meta;

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

    Ok(TokenStream::from(quote! {
        impl near_contract_tools::event::EventMetadata for #type_name {
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
    }))
}
