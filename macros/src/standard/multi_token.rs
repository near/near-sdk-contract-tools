use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Expr, Type};

use crate::unitify;

use super::{nep145, nep245};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(multi_token), supports(struct_named))]
pub struct MultiTokenMeta {
    pub all_hooks: Option<Type>,

    // NEP-145 fields
    pub storage_management_storage_key: Option<Expr>,
    pub force_unregister_hook: Option<Type>,

    // NEP-245 fields
    pub core_storage_key: Option<Expr>,
    pub mint_hook: Option<Type>,
    pub transfer_hook: Option<Type>,
    pub burn_hook: Option<Type>,
    pub load_token_metadata: Option<Type>,

    // NEP-245 metadata fields
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

pub fn expand(meta: MultiTokenMeta) -> Result<TokenStream, darling::Error> {
    let MultiTokenMeta {
        all_hooks,

        storage_management_storage_key,
        force_unregister_hook,

        core_storage_key,
        mint_hook,
        transfer_hook,
        burn_hook,
        load_token_metadata,

        metadata_storage_key,

        generics,
        ident,

        me,
        near_sdk,
    } = meta;

    let all_hooks_inner = unitify(all_hooks.clone());
    let force_unregister_hook = unitify(force_unregister_hook);

    let expand_nep145 = nep145::expand(nep145::Nep145Meta {
        storage_key: storage_management_storage_key,
        all_hooks: Some(all_hooks_inner.clone()),
        force_unregister_hook: Some(
            parse_quote! { (#force_unregister_hook, #me::standard::nep245::hooks::BurnNep245OnForceUnregisterHook) },
        ),
        generics: generics.clone(),
        ident: ident.clone(),
        me: me.clone(),
        near_sdk: near_sdk.clone(),
    });

    let expand_nep245 = nep245::expand(nep245::Nep245Meta {
        storage_key: core_storage_key,
        all_hooks: Some(parse_quote! { (
            #all_hooks_inner,
            #me::standard::nep145::hooks::Nep245StorageAccountingHook,
        ) }),
        mint_hook,
        transfer_hook,
        burn_hook,
        load_token_metadata,

        generics: generics.clone(),
        ident: ident.clone(),

        me: me.clone(),
        near_sdk: near_sdk.clone(),
    });

    let expand_nep245_metadata = nep245::metadata::expand(nep245::metadata::Nep245MetadataMeta {
        storage_key: metadata_storage_key,

        generics: generics.clone(),
        ident: ident.clone(),

        me: me.clone(),
        near_sdk: near_sdk.clone(),
    });

    let mut e = darling::Error::accumulator();

    let nep145 = e.handle(expand_nep145);
    let nep245 = e.handle(expand_nep245);
    let nep245_metadata = e.handle(expand_nep245_metadata);

    e.finish_with(quote! {
        #nep145
        #nep245
        #nep245_metadata
    })
}
