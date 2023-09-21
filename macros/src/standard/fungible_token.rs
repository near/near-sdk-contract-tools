use darling::{util::Flag, FromDeriveInput};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

use super::{nep141, nep148};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(fungible_token), supports(struct_named))]
pub struct FungibleTokenMeta {
    // NEP-141 fields
    pub core_storage_key: Option<Expr>,
    pub no_hooks: Flag,

    // NEP-148 fields
    pub metadata_storage_key: Option<Expr>,

    // darling
    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: FungibleTokenMeta) -> Result<TokenStream, darling::Error> {
    let FungibleTokenMeta {
        core_storage_key,
        metadata_storage_key,
        no_hooks,

        generics,
        ident,

        me,
        near_sdk,
    } = meta;

    let expand_nep141 = nep141::expand(nep141::Nep141Meta {
        storage_key: core_storage_key,
        no_hooks,

        generics: generics.clone(),
        ident: ident.clone(),

        me: me.clone(),
        near_sdk: near_sdk.clone(),
    });

    let expand_nep148 = nep148::expand(nep148::Nep148Meta {
        storage_key: metadata_storage_key,
        generics,
        ident,

        me,
        near_sdk,
    });

    let mut e = darling::Error::accumulator();

    let nep141 = e.handle(expand_nep141);
    let nep148 = e.handle(expand_nep148);

    e.finish_with(quote! {
        #nep141
        #nep148
    })
}
