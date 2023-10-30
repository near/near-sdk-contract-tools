use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, Expr, Type};

use crate::unitify;

use super::{nep145, nep171, nep177, nep178, nep181};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(non_fungible_token), supports(struct_named))]
pub struct NonFungibleTokenMeta {
    pub all_hooks: Option<Type>,

    // NEP-145 fields
    pub storage_management_storage_key: Option<Expr>,
    pub force_unregister_hook: Option<Type>,

    // NEP-171 fields
    pub core_storage_key: Option<Expr>,
    pub mint_hook: Option<Type>,
    pub transfer_hook: Option<Type>,
    pub burn_hook: Option<Type>,

    // NEP-177 fields
    pub metadata_storage_key: Option<Expr>,

    // NEP-178 fields
    pub approval_storage_key: Option<Expr>,
    pub approve_hook: Option<Type>,
    pub revoke_hook: Option<Type>,
    pub revoke_all_hook: Option<Type>,

    // NEP-181 fields
    pub enumeration_storage_key: Option<Expr>,

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
        all_hooks,

        storage_management_storage_key,
        force_unregister_hook,

        core_storage_key,
        mint_hook,
        transfer_hook,
        burn_hook,

        metadata_storage_key,

        approval_storage_key,
        approve_hook,
        revoke_hook,
        revoke_all_hook,

        enumeration_storage_key,

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
            parse_quote! { (#force_unregister_hook, #me::standard::nep171::hooks::BurnNep171OnForceUnregisterHook) },
        ),
        generics: generics.clone(),
        ident: ident.clone(),
        me: me.clone(),
        near_sdk: near_sdk.clone(),
    });

    let expand_nep171 = nep171::expand(nep171::Nep171Meta {
        storage_key: core_storage_key,
        all_hooks: Some(parse_quote! { (
            #all_hooks_inner,
            (
                #me::standard::nep145::hooks::Nep171StorageAccountingHook,
                (
                    #me::standard::nep178::TokenApprovals,
                    #me::standard::nep181::TokenEnumeration,
                ),
            ),
        ) }),
        mint_hook,
        transfer_hook,
        burn_hook,
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
        all_hooks,
        approve_hook,
        revoke_hook,
        revoke_all_hook,

        generics: generics.clone(),
        ident: ident.clone(),
        me: me.clone(),
        near_sdk: near_sdk.clone(),
    });

    let expand_nep181 = nep181::expand(nep181::Nep181Meta {
        storage_key: enumeration_storage_key,
        generics,
        ident,
        me,
        near_sdk,
    });

    let mut e = darling::Error::accumulator();

    let nep145 = e.handle(expand_nep145);
    let nep171 = e.handle(expand_nep171);
    let nep177 = e.handle(expand_nep177);
    let nep178 = e.handle(expand_nep178);
    let nep181 = e.handle(expand_nep181);

    e.finish_with(quote! {
        #nep145
        #nep171
        #nep177
        #nep178
        #nep181
    })
}
