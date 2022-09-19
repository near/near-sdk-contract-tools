use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;

use crate::rename::RenameStrategy;

#[derive(Debug, FromMeta)]
pub struct EventAttributeMeta {
    pub standard: String,
    pub version: String,
    pub rename_all: Option<RenameStrategy>,

    /// serde crate path
    pub serde: Option<syn::Expr>,
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
        #[derive(near_contract_tools::Nep297, #serde :: Serialize)]
        #[nep297(standard = #standard, version = #version, rename_all = #rename_all)]
        #[serde(crate = #serde_str, untagged)]
        #item
    }
}
