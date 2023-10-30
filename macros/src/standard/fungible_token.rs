use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, Type};

use super::{nep141, nep145, nep148};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(fungible_token), supports(struct_named))]
pub struct FungibleTokenMeta {
    // NEP-141 fields
    pub core_storage_key: Option<Expr>,
    pub all_hooks: Option<Type>,
    pub mint_hook: Option<Type>,
    pub transfer_hook: Option<Type>,
    pub burn_hook: Option<Type>,

    // NEP-148 fields
    pub metadata_storage_key: Option<Expr>,

    // NEP-145 fields
    pub storage_management_storage_key: Option<Expr>,
    pub force_unregister_hook: Option<Type>,

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
        all_hooks,
        mint_hook,
        transfer_hook,
        burn_hook,

        metadata_storage_key,

        storage_management_storage_key,
        force_unregister_hook,

        generics,
        ident,

        me,
        near_sdk,
    } = meta;

    let all_hooks_or_unit = all_hooks
        .clone()
        .unwrap_or_else(|| syn::parse_quote! { () });
    let force_unregister_hook_or_unit =
        force_unregister_hook.unwrap_or_else(|| syn::parse_quote! { () });

    let expand_nep141 = nep141::expand(nep141::Nep141Meta {
        storage_key: core_storage_key,
        all_hooks: Some(
            syn::parse_quote! { (#all_hooks_or_unit, #me::standard::nep145::hooks::Nep141StorageAccountingHook) },
        ),
        mint_hook,
        transfer_hook,
        burn_hook,

        generics: generics.clone(),
        ident: ident.clone(),

        me: me.clone(),
        near_sdk: near_sdk.clone(),
    });

    let expand_nep145 = nep145::expand(nep145::Nep145Meta {
        storage_key: storage_management_storage_key,
        all_hooks,
        force_unregister_hook: Some(
            syn::parse_quote! { (#force_unregister_hook_or_unit, #me::standard::nep141::hooks::BurnNep141OnForceUnregisterHook) },
        ),
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
    let nep145 = e.handle(expand_nep145);
    let nep148 = e.handle(expand_nep148);

    e.finish_with(quote! {
        #nep141
        #nep145
        #nep148
    })
}
