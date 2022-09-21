use darling::{util::Flag, FromMeta};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Item;

use crate::rename::RenameStrategy;

#[derive(Debug, FromMeta)]
pub struct EventAttributeMeta {
    pub standard: String,
    pub version: String,
    pub rename: Option<RenameStrategy>,
    pub name: Option<String>,
    pub batch: Flag,

    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_macros")]
    pub macros: syn::Path,
    #[darling(default = "crate::default_serde")]
    pub serde: syn::Path,
}

pub fn event_attribute(
    attr: EventAttributeMeta,
    item: Item,
) -> Result<TokenStream, darling::Error> {
    let EventAttributeMeta {
        standard,
        version,
        rename,
        name,
        batch,
        serde,
        me,
        macros,
    } = attr;

    let rename = rename.unwrap_or(RenameStrategy::SnakeCase).to_string();
    let name = name.map(|n| quote! { , name = #n });

    let serde_str = quote! { #serde }.to_string();
    let me_str = quote! { #me }.to_string();

    let batch = batch.is_present().then_some(quote! {, batch});

    Ok(quote::quote! {
        #[derive(#macros::Nep297, #serde::Serialize)]
        #[nep297(standard = #standard, version = #version, rename = #rename, crate = #me_str #name #batch)]
        #[serde(crate = #serde_str)]
        #item
    })
}
