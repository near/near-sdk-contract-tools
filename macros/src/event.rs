use darling::{ast::Style, FromDeriveInput, FromMeta, FromVariant};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

use crate::rename::RenameStrategy;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(event), supports(enum_any))]
pub struct EventMeta {
    pub standard: String,
    pub version: String,
    pub rename_all: Option<RenameStrategy>,

    pub ident: syn::Ident,
    pub generics: syn::Generics,
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

pub fn expand(meta: EventMeta) -> Result<TokenStream, darling::Error> {
    let EventMeta {
        standard,
        version,
        rename_all,
        ident: type_name,
        generics,
        data,
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
        impl #imp near_contract_tools::event::EventMetadata for #type_name #ty #wher {
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
                    near_contract_tools::event::Event::to_event_string(self),
                )
            }
        }
    })
}

#[derive(Debug, FromMeta)]
pub struct EventAttributeMeta {
    pub standard: String,
    pub version: String,
    pub rename_all: Option<RenameStrategy>,

    /// serde crate path
    pub serde: Option<Expr>,
}

pub fn event_attribute(attr: EventAttributeMeta, item: TokenStream) -> TokenStream {
    let EventAttributeMeta {
        standard,
        version,
        rename_all,
        serde,
    } = attr;

    let rename_all = rename_all.unwrap_or(RenameStrategy::SnakeCase).to_string();

    let serde = serde
        .map(|s| quote! { #s })
        .unwrap_or_else(|| quote! { ::serde });

    let serde_str = serde.to_string();

    quote::quote! {
        #[derive(::near_contract_tools::Event, #serde :: Serialize)]
        #[event(standard = #standard, version = #version, rename_all = #rename_all)]
        #[serde(crate = #serde_str, untagged)]
        #item
    }
}
