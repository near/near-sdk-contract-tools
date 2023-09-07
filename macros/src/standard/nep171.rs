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
    pub extension_hooks: Option<syn::Type>,
    pub check_external_transfer: Option<syn::Type>,
    pub token_data: Option<syn::Type>,

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
        extension_hooks,
        check_external_transfer,
        token_data,

        generics,
        ident,

        me,
        near_sdk,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    let token_data = token_data
        .map(|token_data| quote! { #token_data })
        .unwrap_or_else(|| {
            quote! { () }
        });

    let check_external_transfer = check_external_transfer
        .map(|check_external_transfer| quote! { #check_external_transfer })
        .unwrap_or_else(|| {
            quote! { #me::standard::nep171::DefaultCheckExternalTransfer }
        });

    let root = storage_key.map(|storage_key| {
        quote! {
            fn root() -> #me::slot::Slot<()> {
                #me::slot::Slot::root(#storage_key)
            }
        }
    });

    let extension_hooks_type = extension_hooks
        .map(|extension_hooks| quote! { #extension_hooks })
        .unwrap_or_else(|| {
            quote! { () }
        });

    let self_hooks_type = no_hooks
        .is_present()
        .not()
        .then(|| {
            quote! {
                Self
            }
        })
        .unwrap_or_else(|| {
            quote! {
                ()
            }
        });

    let hooks_type = quote! { (#self_hooks_type, #extension_hooks_type) };

    Ok(quote! {
        impl #imp #me::standard::nep171::Nep171ControllerInternal for #ident #ty #wher {
            type Hook = #hooks_type;
            type CheckExternalTransfer = #check_external_transfer;
            type LoadTokenMetadata = #token_data;

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
                use #me::standard::nep171::*;

                let _ = approved_account_ids; // #[near_bindgen] cares about parameter names

                #near_sdk::require!(
                    #near_sdk::env::promise_results_count() == 1,
                    "Requires exactly one promise result.",
                );

                let should_revert =
                    if let #near_sdk::PromiseResult::Successful(value) = #near_sdk::env::promise_result(0) {
                        let value = #near_sdk::serde_json::from_slice::<bool>(&value).unwrap_or(true);
                        value
                    } else {
                        true
                    };

                if should_revert {
                    let token_ids = [token_id];

                    let transfer = Nep171Transfer {
                        token_id: &token_ids[0],
                        authorization: Nep171TransferAuthorization::Owner,
                        sender_id: Some(&receiver_id),
                        receiver_id: &previous_owner_id,
                        memo: None,
                        msg: None,
                        revert: true,
                    };

                    <Self as Nep171Controller>::external_transfer(self, &transfer)
                        .is_err()
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
                approval_id: Option<u32>,
                memo: Option<String>,
            ) {
                use #me::standard::nep171::*;

                #near_sdk::assert_one_yocto();

                let sender_id = #near_sdk::env::predecessor_account_id();

                let token_ids = [token_id];

                let transfer = Nep171Transfer {
                    token_id: &token_ids[0],
                    authorization: approval_id.map(Nep171TransferAuthorization::ApprovalId).unwrap_or(Nep171TransferAuthorization::Owner),
                    sender_id: Some(&sender_id),
                    receiver_id: &receiver_id,
                    memo: memo.as_deref(),
                    msg: None,
                    revert: false,
                };

                <Self as Nep171Controller>::external_transfer(self, &transfer)
                    .unwrap_or_else(|e| #near_sdk::env::panic_str(&e.to_string()));
            }

            #[payable]
            fn nft_transfer_call(
                &mut self,
                receiver_id: #near_sdk::AccountId,
                token_id: #me::standard::nep171::TokenId,
                approval_id: Option<u32>,
                memo: Option<String>,
                msg: String,
            ) -> #near_sdk::PromiseOrValue<bool> {
                use #me::standard::nep171::*;

                #near_sdk::assert_one_yocto();

                #near_sdk::require!(
                    #near_sdk::env::prepaid_gas() >= GAS_FOR_NFT_TRANSFER_CALL,
                    INSUFFICIENT_GAS_MESSAGE,
                );

                let sender_id = #near_sdk::env::predecessor_account_id();

                let token_ids = [token_id];

                let transfer = Nep171Transfer {
                    token_id: &token_ids[0],
                    authorization: approval_id.map(Nep171TransferAuthorization::ApprovalId).unwrap_or(Nep171TransferAuthorization::Owner),
                    sender_id: Some(&sender_id),
                    receiver_id: &receiver_id,
                    memo: memo.as_deref(),
                    msg: Some(&msg),
                    revert: false,
                };

                <Self as Nep171Controller>::external_transfer(self, &transfer)
                    .unwrap_or_else(|e| #near_sdk::env::panic_str(&e.to_string()));

                let [token_id] = token_ids;

                ext_nep171_receiver::ext(receiver_id.clone())
                    .with_static_gas(#near_sdk::env::prepaid_gas() - GAS_FOR_NFT_TRANSFER_CALL)
                    .nft_on_transfer(
                        sender_id.clone(),
                        sender_id.clone(),
                        token_id.clone(),
                        msg.clone(),
                    )
                    .then(
                        ext_nep171_resolver::ext(#near_sdk::env::current_account_id())
                            .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                            .nft_resolve_transfer(sender_id.clone(), receiver_id.clone(), token_id.clone(), None),
                    )
                    .into()
            }

            fn nft_token(
                &self,
                token_id: #me::standard::nep171::TokenId,
            ) -> Option<#me::standard::nep171::Token> {
                <Self as #me::standard::nep171::Nep171Controller>::load_token(self, &token_id)
            }
        }
    })
}
