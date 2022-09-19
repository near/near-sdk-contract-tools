use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;

use crate::rename::RenameStrategy;

#[derive(Debug, FromMeta)]
pub struct EventAttributeMeta {
    pub standard: String,
    pub version: String,
    pub rename_all: Option<RenameStrategy>,

    // pub me: String,
    // pub macros: String,
    // pub serde: String,
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_macros")]
    pub macros: syn::Path,
    #[darling(default = "crate::default_serde")]
    pub serde: syn::Path,
}

pub fn event_attribute(
    attr: EventAttributeMeta,
    item: TokenStream,
) -> Result<TokenStream, darling::Error> {
    let EventAttributeMeta {
        standard,
        version,
        rename_all,
        serde,
        me,
        macros,
    } = attr;

    let rename_all = rename_all.unwrap_or(RenameStrategy::SnakeCase).to_string();

    let serde_str = quote! { #serde }.to_string();
    let me_str = quote! { #me }.to_string();

    Ok(quote::quote! {
        #[derive(#macros::Nep297, #serde::Serialize)]
        #[nep297(standard = #standard, version = #version, rename_all = #rename_all, crate = #me_str)]
        #[serde(crate = #serde_str, untagged)]
        #item
    })
}
