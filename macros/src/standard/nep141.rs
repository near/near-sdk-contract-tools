use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, Type};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep141), supports(struct_named))]
pub struct Nep141Meta {
    pub storage_key: Option<Expr>,
    pub all_hooks: Option<Type>,
    pub mint_hook: Option<Type>,
    pub transfer_hook: Option<Type>,
    pub burn_hook: Option<Type>,
    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: Nep141Meta) -> Result<TokenStream, darling::Error> {
    let Nep141Meta {
        storage_key,
        all_hooks,
        mint_hook,
        transfer_hook,
        burn_hook,
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

    let mint_hook = mint_hook
        .map(|h| quote! { #h })
        .unwrap_or_else(|| quote! { () });
    let transfer_hook = transfer_hook
        .map(|h| quote! { #h })
        .unwrap_or_else(|| quote! { () });
    let burn_hook = burn_hook
        .map(|h| quote! { #h })
        .unwrap_or_else(|| quote! { () });

    let default_hook = all_hooks
        .map(|h| quote! { #h })
        .unwrap_or_else(|| quote! { () });

    Ok(quote! {
        impl #imp #me::standard::nep141::Nep141ControllerInternal for #ident #ty #wher {
            type MintHook = (#mint_hook, #default_hook);
            type TransferHook = (#transfer_hook, #default_hook);
            type BurnHook = (#burn_hook, #default_hook);

            #root
        }

        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep141::Nep141 for #ident #ty #wher {
            #[payable]
            fn ft_transfer(
                &mut self,
                receiver_id: #near_sdk::AccountId,
                amount: #near_sdk::json_types::U128,
                memo: Option<String>,
            ) {
                use #me::standard::nep141::*;

                #near_sdk::assert_one_yocto();
                let sender_id = #near_sdk::env::predecessor_account_id();
                let amount: u128 = amount.into();

                let transfer = Nep141Transfer {
                    sender_id: &sender_id,
                    receiver_id: &receiver_id,
                    amount,
                    memo: memo.as_deref(),
                    msg: None,
                    revert: false,
                };

                Nep141Controller::transfer(self, &transfer)
                    .unwrap_or_else(|e| #near_sdk::env::panic_str(&e.to_string()));
            }

            #[payable]
            fn ft_transfer_call(
                &mut self,
                receiver_id: #near_sdk::AccountId,
                amount: #near_sdk::json_types::U128,
                memo: Option<String>,
                msg: String,
            ) -> #near_sdk::Promise {
                use #me::standard::nep141::*;

                let prepaid_gas = #near_sdk::env::prepaid_gas();

                #near_sdk::require!(
                    prepaid_gas >= GAS_FOR_FT_TRANSFER_CALL,
                    MORE_GAS_FAIL_MESSAGE,
                );

                #near_sdk::assert_one_yocto();
                let sender_id = #near_sdk::env::predecessor_account_id();
                let amount: u128 = amount.into();

                let transfer = Nep141Transfer {
                    sender_id: &sender_id,
                    receiver_id: &receiver_id,
                    amount,
                    memo: memo.as_deref(),
                    msg: Some(&msg),
                    revert: false,
                };

                Nep141Controller::transfer(self, &transfer)
                    .unwrap_or_else(|e| #near_sdk::env::panic_str(&e.to_string()));

                let receiver_gas = #me::compat_gas_to_u64!(prepaid_gas)
                    .checked_sub(#me::compat_gas_to_u64!(GAS_FOR_FT_TRANSFER_CALL))
                    .unwrap_or_else(|| #near_sdk::env::panic_str("Prepaid gas underflow."));

                // Initiating receiver's call and the callback
                ext_nep141_receiver::ext(transfer.receiver_id.clone())
                    .with_static_gas(#me::compat_gas!(receiver_gas))
                    .ft_on_transfer(transfer.sender_id.clone(), transfer.amount.into(), msg.clone())
                    .then(
                        ext_nep141_resolver::ext(#near_sdk::env::current_account_id())
                            .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                            .ft_resolve_transfer(
                                transfer.sender_id.clone(),
                                transfer.receiver_id.clone(),
                                transfer.amount.into(),
                            ),
                    )
            }

            fn ft_total_supply(&self) -> #near_sdk::json_types::U128 {
                #me::standard::nep141::Nep141Controller::total_supply(self).into()
            }

            fn ft_balance_of(&self, account_id: #near_sdk::AccountId) -> #near_sdk::json_types::U128 {
                #me::standard::nep141::Nep141Controller::balance_of(self, &account_id).into()
            }
        }

        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep141::Nep141Resolver for #ident #ty #wher {
            #[private]
            fn ft_resolve_transfer(
                &mut self,
                sender_id: #near_sdk::AccountId,
                receiver_id: #near_sdk::AccountId,
                amount: #near_sdk::json_types::U128,
            ) -> #near_sdk::json_types::U128 {
                use #near_sdk::{env, PromiseResult, serde_json, json_types::U128};
                use #me::standard::nep141::*;

                let amount = amount.0;

                let ft_on_transfer_promise_result = env::promise_result(0);

                let unused_amount = match ft_on_transfer_promise_result {
                    PromiseResult::Successful(value) => {
                        if let Ok(U128(unused_amount)) = serde_json::from_slice::<U128>(&value) {
                            std::cmp::min(amount, unused_amount)
                        } else {
                            amount
                        }
                    }
                    PromiseResult::Failed => amount,
                    _ => env::abort(),
                };

                let refunded_amount = if unused_amount > 0 {
                    let receiver_balance = Nep141Controller::balance_of(self, &receiver_id);
                    if receiver_balance > 0 {
                        let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                        let transfer = Nep141Transfer {
                            sender_id: &receiver_id,
                            receiver_id: &sender_id,
                            amount: refund_amount,
                            memo: None,
                            msg: None,
                            revert: true,
                        };

                        Nep141Controller::transfer(self, &transfer)
                            .unwrap_or_else(|e| env::panic_str(&e.to_string()));

                        refund_amount
                    } else {
                        0
                    }
                } else {
                    0
                };

                // Used amount
                U128(amount - refunded_amount)
            }
        }
    })
}
