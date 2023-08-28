use darling::{util::Flag, FromDeriveInput};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

use super::{nep171, nep177, nep178};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(non_fungible_token), supports(struct_named))]
pub struct NonFungibleTokenMeta {
    // NEP-171 fields
    pub core_storage_key: Option<Expr>,
    pub no_core_hooks: Flag,

    // NEP-177 fields
    pub metadata_storage_key: Option<Expr>,

    // NEP-178 fields
    pub approval_storage_key: Option<Expr>,
    pub no_approval_hooks: Flag,

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
        core_storage_key: storage_key,
        no_core_hooks: no_hooks,

        metadata_storage_key,

        approval_storage_key,
        no_approval_hooks,

        generics,
        ident,

        me,
        near_sdk,
    } = meta;

    let expand_nep171 = nep171::expand(nep171::Nep171Meta {
        storage_key,
        no_hooks,
        extension_hooks: Some(syn::parse_quote! { #me::standard::nep178::TokenApprovals }),
        check_external_transfer: Some(syn::parse_quote! { #me::standard::nep178::TokenApprovals }),

        token_data: Some(
            syn::parse_quote! { (#me::standard::nep177::TokenMetadata, #me::standard::nep178::TokenApprovals) },
        ),

        generics: generics.clone(),
        ident: ident.clone(),

        me: me.clone(),
        near_sdk: near_sdk.clone(),
    });

    let expand_nep177 = nep177::expand(nep177::Nep177Meta {
        storage_key: metadata_storage_key,

        generics: generics.clone(),
        ident: ident.clone(),

        me: me.clone(),
        near_sdk: near_sdk.clone(),
    });

    let expand_nep178 = nep178::expand(nep178::Nep178Meta {
        storage_key: approval_storage_key,
        no_hooks: no_approval_hooks,
        generics,
        ident,
        me,
        near_sdk,
    });

    let mut e = darling::Error::accumulator();

    let nep171 = e.handle(expand_nep171);
    let nep177 = e.handle(expand_nep177);
    let nep178 = e.handle(expand_nep178);

    e.finish_with(quote! {
        #nep171
        #nep177
        #nep178
    })
}
