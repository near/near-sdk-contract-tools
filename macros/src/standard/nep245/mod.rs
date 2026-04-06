pub mod metadata;

use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, Type};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep245), supports(struct_named))]
pub struct Nep245Meta {
    pub storage_key: Option<Expr>,
    pub all_hooks: Option<Type>,
    pub mint_hook: Option<Type>,
    pub transfer_hook: Option<Type>,
    pub burn_hook: Option<Type>,
    pub load_token_metadata: Option<Type>,
    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: Nep245Meta) -> Result<TokenStream, darling::Error> {
    let Nep245Meta {
        storage_key,
        all_hooks,
        mint_hook,
        transfer_hook,
        burn_hook,
        load_token_metadata,
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

    let mint_hook = mint_hook.map_or_else(|| quote! { () }, |h| quote! { #h });
    let transfer_hook = transfer_hook.map_or_else(|| quote! { () }, |h| quote! { #h });
    let burn_hook = burn_hook.map_or_else(|| quote! { () }, |h| quote! { #h });
    let load_token_metadata = load_token_metadata.map_or_else(|| quote! { () }, |h| quote! { #h });

    let default_hook = all_hooks.map_or_else(|| quote! { () }, |h| quote! { #h });

    Ok(quote! {
        impl #imp #me::standard::nep245::Nep245ControllerInternal for #ident #ty #wher {
            type MintHook = (#mint_hook, #default_hook);
            type TransferHook = (#transfer_hook, #default_hook);
            type BurnHook = (#burn_hook, #default_hook);
            type LoadTokenMetadata = #load_token_metadata;

            #root
        }

        #[#near_sdk::near]
        impl #imp #me::standard::nep245::Nep245 for #ident #ty #wher {
            #[payable]
            fn mt_transfer(
                &mut self,
                receiver_id: #near_sdk::AccountId,
                token_id: #me::standard::nep245::TokenId,
                amount: #near_sdk::json_types::U128,
                approval: Option<#me::standard::nep245::MtTransferApproval>,
                memo: Option<String>,
            ) {
                let _ = approval;

                use #me::standard::nep245::*;

                #near_sdk::assert_one_yocto();
                let sender_id = #near_sdk::env::predecessor_account_id();
                let amount: u128 = amount.into();

                let transfer = Nep245Transfer::single(
                    sender_id,
                    receiver_id,
                    token_id,
                    amount,
                    memo,
                );

                Nep245Controller::transfer(self, &transfer)
                    .unwrap_or_else(|e| #near_sdk::env::panic_str(&e.to_string()));
            }

            #[payable]
            fn mt_batch_transfer(
                &mut self,
                receiver_id: #near_sdk::AccountId,
                token_ids: Vec<#me::standard::nep245::TokenId>,
                amounts: Vec<#near_sdk::json_types::U128>,
                approvals: Option<Vec<Option<#me::standard::nep245::MtTransferApproval>>>,
                memo: Option<String>,
            ) {
                let _ = approvals;

                use #me::standard::nep245::*;

                #near_sdk::assert_one_yocto();
                let sender_id = #near_sdk::env::predecessor_account_id();

                let count = token_ids.len();
                let transfer = Nep245Transfer::new(
                    sender_id,
                    receiver_id,
                    count,
                    token_ids,
                    amounts,
                    memo,
                ).unwrap_or_else(|e| {
                    #near_sdk::env::panic_str(&e.to_string())
                });

                Nep245Controller::transfer(self, &transfer)
                    .unwrap_or_else(|e| #near_sdk::env::panic_str(&e.to_string()));
            }

            #[payable]
            fn mt_transfer_call(
                &mut self,
                receiver_id: #near_sdk::AccountId,
                token_id: #me::standard::nep245::TokenId,
                amount: #near_sdk::json_types::U128,
                approval: Option<#me::standard::nep245::MtTransferApproval>,
                memo: Option<String>,
                msg: String,
            ) -> #near_sdk::Promise {
                let _ = approval;

                use #me::standard::nep245::*;

                #near_sdk::require!(
                    #near_sdk::env::prepaid_gas() >= GAS_FOR_MT_TRANSFER_CALL,
                    INSUFFICIENT_GAS_MESSAGE,
                );

                #near_sdk::assert_one_yocto();
                let sender_id = #near_sdk::env::predecessor_account_id();
                let amount: u128 = amount.into();

                let transfer = Nep245Transfer::single(
                    sender_id,
                    receiver_id,
                    token_id,
                    amount,
                    memo,
                ).msg(msg.clone());

                Nep245Controller::transfer(self, &transfer)
                    .unwrap_or_else(|e| #near_sdk::env::panic_str(&e.to_string()));

                transfer.promise(#near_sdk::env::current_account_id())
            }

            #[payable]
            fn mt_batch_transfer_call(
                &mut self,
                receiver_id: #near_sdk::AccountId,
                token_ids: Vec<#me::standard::nep245::TokenId>,
                amounts: Vec<#near_sdk::json_types::U128>,
                approvals: Option<Vec<Option<#me::standard::nep245::MtTransferApproval>>>,
                memo: Option<String>,
                msg: String,
            ) -> #near_sdk::Promise {
                let _ = approvals;

                use #me::standard::nep245::*;

                #near_sdk::require!(
                    #near_sdk::env::prepaid_gas() >= GAS_FOR_MT_TRANSFER_CALL,
                    INSUFFICIENT_GAS_MESSAGE,
                );

                #near_sdk::assert_one_yocto();
                let sender_id = #near_sdk::env::predecessor_account_id();

                let count = token_ids.len();
                let transfer = Nep245Transfer::new(
                        sender_id,
                        receiver_id,
                        count,
                        token_ids,
                        amounts,
                        memo,
                    )
                    .unwrap_or_else(|e| {
                        #near_sdk::env::panic_str(&e.to_string())
                    })
                    .msg(msg.clone());

                Nep245Controller::transfer(self, &transfer)
                    .unwrap_or_else(|e| #near_sdk::env::panic_str(&e.to_string()));

                transfer.promise(#near_sdk::env::current_account_id())
            }

            fn mt_token(
                &self,
                token_ids: Vec<#me::standard::nep245::TokenId>,
            ) -> Vec<Option<#me::standard::nep245::Token>> {
                token_ids.into_iter().map(|token_id| {
                    #me::standard::nep245::Nep245Controller::token(self, &token_id)
                })
                .collect()
            }

            fn mt_balance_of(
                &self,
                account_id: #near_sdk::AccountId,
                token_id: #me::standard::nep245::TokenId,
            ) -> #near_sdk::json_types::U128 {
                #me::standard::nep245::Nep245Controller::balance_of(self, &token_id, &account_id).into()
            }

            fn mt_batch_balance_of(
                &self,
                account_id: #near_sdk::AccountId,
                token_ids: Vec<#me::standard::nep245::TokenId>,
            ) -> Vec<#near_sdk::json_types::U128> {
                token_ids
                    .into_iter()
                    .map(|token_id| {
                        #me::standard::nep245::Nep245Controller::balance_of(self, &token_id, &account_id).into()
                    })
                    .collect()
            }

            fn mt_supply(
                &self,
                token_id: #me::standard::nep245::TokenId,
            ) -> Option<#near_sdk::json_types::U128> {
                #me::standard::nep245::Nep245Controller::supply(
                    self,
                    &token_id,
                )
                .map(Into::into)
            }

            fn mt_batch_supply(
                &self,
                token_ids: Vec<#me::standard::nep245::TokenId>,
            ) -> Vec<Option<#near_sdk::json_types::U128>> {
                token_ids
                    .into_iter()
                    .map(|token_id| {
                        #me::standard::nep245::Nep245Controller::supply(
                            self,
                            &token_id,
                        )
                        .map(Into::into)
                    })
                    .collect()
            }
        }

        #[#near_sdk::near]
        impl #imp #me::standard::nep245::Nep245Resolver for #ident #ty #wher {
            #[private]
            fn mt_resolve_transfer(
                &mut self,
                sender_id: #near_sdk::AccountId,
                receiver_id: #near_sdk::AccountId,
                token_ids: Vec<#me::standard::nep245::TokenId>,
                amounts: Vec<#near_sdk::json_types::U128>,
            ) -> Vec<#near_sdk::json_types::U128> {
                use #near_sdk::{env, PromiseResult, serde_json, json_types::U128};
                use #me::standard::nep245::*;

                let mt_on_transfer_result = match env::promise_result(0) {
                    PromiseResult::Successful(value) => {
                        serde_json::from_slice::<Vec<U128>>(&value).ok()
                    }
                    PromiseResult::Failed => None,
                    _ => env::abort(),
                };

                Nep245Controller::resolve_transfer(
                    self,
                    sender_id,
                    receiver_id,
                    token_ids,
                    amounts,
                    mt_on_transfer_result,
                )
            }
        }
    })
}
