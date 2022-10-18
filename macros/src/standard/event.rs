use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Item;

use crate::rename::RenameStrategy;

#[derive(Debug, FromMeta)]
pub struct EventAttributeMeta {
    pub standard: String,
    pub version: String,
    pub rename: Option<RenameStrategy>,
    pub rename_all: Option<RenameStrategy>,
    pub name: Option<String>,

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
        rename_all,
        name,
        serde,
        me,
        macros,
    } = attr;

    let serde_untagged = matches!(item, Item::Enum(_)).then_some(quote! { #[serde(untagged)] });

    let default_rename = if rename.is_none() && rename_all.is_none() {
        Some(match item {
            Item::Enum(_) => quote! { rename_all = "snake_case", },
            Item::Struct(_) => quote! { rename = "snake_case", },
            _ => unreachable!(),
        })
    } else {
        None
    };

    let rename = rename.map(|r| {
        let r = r.to_string();
        quote! { rename = #r, }
    });
    let rename_all = rename_all.map(|r| {
        let r = r.to_string();
        quote! { rename_all = #r, }
    });

    let name = name.map(|n| quote! { name = #n, });

    let serde_str = quote! { #serde }.to_string();
    let me_str = quote! { #me }.to_string();

    Ok(quote::quote! {
        #[derive(#macros::Nep297, #serde::Serialize)]
        #[nep297(
            crate = #me_str,
            standard = #standard,
            version = #version,
            #rename #rename_all #default_rename #name
        )]
        #[serde(crate = #serde_str)]
        #serde_untagged
        #item
    })
}
