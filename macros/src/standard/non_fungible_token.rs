use darling::{util::Flag, FromDeriveInput};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

use super::{nep171, nep177};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(non_fungible_token), supports(struct_named))]
pub struct NonFungibleTokenMeta {
    // NEP-171 fields
    pub storage_key: Option<Expr>,
    pub no_hooks: Flag,
    pub extension_hooks: Option<syn::Type>,
    pub check_external_transfer: Option<syn::Type>,

    // NEP-177 fields
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

pub fn expand(meta: NonFungibleTokenMeta) -> Result<TokenStream, darling::Error> {
    let NonFungibleTokenMeta {
        storage_key,
        no_hooks,
        extension_hooks,
        check_external_transfer,

        metadata_storage_key,

        generics,
        ident,

        me,
        near_sdk,
    } = meta;

    let expand_nep171 = nep171::expand(nep171::Nep171Meta {
        storage_key,
        no_hooks,
        extension_hooks,
        check_external_transfer,

        token_type: Some(syn::parse_quote! { ( #me::standard::nep177::TokenMetadata ) }),

        generics: generics.clone(),
        ident: ident.clone(),

        me: me.clone(),
        near_sdk: near_sdk.clone(),
    });

    let expand_nep177 = nep177::expand(nep177::Nep177Meta {
        storage_key: metadata_storage_key,

        generics,
        ident,

        me,
        near_sdk,
    });

    let mut e = darling::Error::accumulator();

    let nep171 = e.handle(expand_nep171);
    let nep177 = e.handle(expand_nep177);

    e.finish_with(quote! {
        #nep171
        #nep177
    })
}
