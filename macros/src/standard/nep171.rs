use std::ops::Not;

use darling::{util::Flag, FromDeriveInput};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep171), supports(struct_named))]
pub struct Nep171Meta {
    pub storage_key: Option<Expr>,
    pub no_hooks: Flag,
    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: Nep171Meta) -> Result<TokenStream, darling::Error> {
    let Nep171Meta {
        storage_key,
        no_hooks,
        generics,
        ident,

        me,
        near_sdk,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    let root = storage_key.map(|storage_key| {
        quote! {
            fn root() -> #me::slot::Slot<()> {
                #me::slot::Slot::root(#storage_key)
            }
        }
    });

    let before_nft_transfer = no_hooks.is_present().not().then(|| {
        quote! {
            let hook_state = <Self as #me::standard::nep171::Nep171Hook::<_>>::before_nft_transfer(self, &transfer);
        }
    });

    let after_nft_transfer = no_hooks.is_present().not().then(|| {
        quote! {
            <Self as #me::standard::nep171::Nep171Hook::<_>>::after_nft_transfer(self, &transfer, hook_state);
        }
    });

    Ok(quote! {
        impl #imp #me::standard::nep171::Nep171ControllerInternal for #ident #ty #wher {
            #root
        }

        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep171::Nep171Resolver for #ident #ty #wher {
            #[private]
            fn nft_resolve_transfer(
                &mut self,
                previous_owner_id: #near_sdk::AccountId,
                receiver_id: #near_sdk::AccountId,
                token_id: #me::standard::nep171::TokenId,
                approved_account_ids: Option<std::collections::HashMap<#near_sdk::AccountId, u64>>,
            ) -> bool {
                let _ = approved_account_ids; // #[near_bindgen] cares about parameter names

                #near_sdk::require!(
                    #near_sdk::env::promise_results_count() == 1,
                    "Requires exactly one promise result.",
                );

                let should_revert =
                    if let #near_sdk::PromiseResult::Successful(value) = #near_sdk::env::promise_result(0) {
                        #near_sdk::serde_json::from_slice::<bool>(&value).unwrap_or(true)
                    } else {
                        true
                    };

                if should_revert {
                    let transfer = #me::standard::nep171::Nep171Transfer {
                        token_id: token_id.clone(),
                        owner_id: receiver_id.clone(),
                        sender_id: receiver_id.clone(),
                        receiver_id: previous_owner_id.clone(),
                        approval_id: None,
                        memo: None,
                        msg: None,
                    };

                    #before_nft_transfer

                    let result = #me::standard::nep171::Nep171Controller::transfer(
                        self,
                        &[token_id],
                        receiver_id.clone(),
                        receiver_id,
                        previous_owner_id,
                        None,
                    )
                    .is_err();

                    #after_nft_transfer

                    result
                } else {
                    true
                }
            }
        }

        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep171::Nep171 for #ident #ty #wher {
            #[payable]
            fn nft_transfer(
                &mut self,
                receiver_id: #near_sdk::AccountId,
                token_id: #me::standard::nep171::TokenId,
                approval_id: Option<u64>,
                memo: Option<String>,
            ) {
                use #me::standard::nep171::*;

                #near_sdk::require!(
                    approval_id.is_none(),
                    APPROVAL_MANAGEMENT_NOT_SUPPORTED_MESSAGE,
                );

                #near_sdk::assert_one_yocto();

                let sender_id = #near_sdk::env::predecessor_account_id();

                let transfer = #me::standard::nep171::Nep171Transfer {
                    token_id: token_id.clone(),
                    owner_id: sender_id.clone(),
                    sender_id: sender_id.clone(),
                    receiver_id: receiver_id.clone(),
                    approval_id: None,
                    memo: memo.clone(),
                    msg: None,
                };

                #before_nft_transfer

                Nep171Controller::transfer(
                    self,
                    &[token_id],
                    sender_id.clone(),
                    sender_id,
                    receiver_id,
                    memo,
                )
                .unwrap_or_else(|e| #near_sdk::env::panic_str(&e.to_string()));

                #after_nft_transfer
            }

            #[payable]
            fn nft_transfer_call(
                &mut self,
                receiver_id: #near_sdk::AccountId,
                token_id: #me::standard::nep171::TokenId,
                approval_id: Option<u64>,
                memo: Option<String>,
                msg: String,
            ) -> #near_sdk::PromiseOrValue<bool> {
                use #me::standard::nep171::*;

                #near_sdk::require!(
                    approval_id.is_none(),
                    APPROVAL_MANAGEMENT_NOT_SUPPORTED_MESSAGE,
                );

                #near_sdk::assert_one_yocto();

                #near_sdk::require!(
                    #near_sdk::env::prepaid_gas() >= GAS_FOR_NFT_TRANSFER_CALL,
                    INSUFFICIENT_GAS_MESSAGE,
                );

                let sender_id = #near_sdk::env::predecessor_account_id();

                let transfer = #me::standard::nep171::Nep171Transfer {
                    token_id: token_id.clone(),
                    owner_id: sender_id.clone(),
                    sender_id: sender_id.clone(),
                    receiver_id: receiver_id.clone(),
                    approval_id: None,
                    memo: memo.clone(),
                    msg: Some(msg.clone()),
                };

                #before_nft_transfer

                let token_ids = [token_id];

                Nep171Controller::transfer(
                    self,
                    &token_ids,
                    sender_id.clone(),
                    sender_id.clone(),
                    receiver_id.clone(),
                    memo,
                )
                .unwrap_or_else(|e| #near_sdk::env::panic_str(&e.to_string()));

                let [token_id] = token_ids;

                #after_nft_transfer

                ext_nep171_receiver::ext(receiver_id.clone())
                    .with_static_gas(#near_sdk::env::prepaid_gas() - GAS_FOR_NFT_TRANSFER_CALL)
                    .nft_on_transfer(
                        sender_id.clone(),
                        sender_id.clone(),
                        token_id.clone(),
                        msg,
                    )
                    .then(
                        ext_nep171_resolver::ext(#near_sdk::env::current_account_id())
                            .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                            .nft_resolve_transfer(sender_id, receiver_id, token_id, None),
                    )
                    .into()
            }

            fn nft_token(
                &self,
                token_id: #me::standard::nep171::TokenId,
            ) -> Option<#me::standard::nep171::Token> {
                use #me::standard::nep171::*;

                Nep171Controller::token_owner(self, &token_id)
                    .map(|owner_id| Token { token_id, owner_id })
            }
        }
    })
}
