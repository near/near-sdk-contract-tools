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

    let before_transfer = no_hooks.is_present().not().then(|| {
        quote! {
            let hook_state = <Self as #me::standard::nep171::Nep171Hook::<_>>::before_transfer(self, &transfer);
        }
    });

    let after_transfer = no_hooks.is_present().not().then(|| {
        quote! {
            <Self as #me::standard::nep171::Nep171Hook::<_>>::after_transfer(self, &transfer, hook_state);
        }
    });

    Ok(quote! {
        impl #imp #me::standard::nep171::Nep171ControllerInternal for #ident #ty #wher {
            #root
        }

        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep171::Nep171 for #ident #ty #wher {
            #[payable]
            fn nft_transfer(
                &mut self,
                receiver_id: #near_sdk::AccountId,
                token_id: String,
                approval_id: Option<u64>,
                memo: Option<String>,
            ) {
                use #me::{
                    standard::{
                        nep171::{Nep171Controller, event},
                        nep297::Event,
                    },
                };

                #near_sdk::assert_one_yocto();
                let sender_id = #near_sdk::env::predecessor_account_id();
                let amount: u128 = amount.into();

                let transfer = #me::standard::nep171::Nep171Transfer {
                    sender_id: sender_id.clone(),
                    receiver_id: receiver_id.clone(),
                    amount,
                    memo: memo.clone(),
                    msg: None,
                };

                #before_transfer

                Nep171Controller::transfer(
                    self,
                    sender_id.clone(),
                    receiver_id.clone(),
                    amount,
                    memo,
                );

                #after_transfer
            }

            #[payable]
            fn ft_transfer_call(
                &mut self,
                receiver_id: #near_sdk::AccountId,
                amount: #near_sdk::json_types::U128,
                memo: Option<String>,
                msg: String,
            ) -> #near_sdk::Promise {
                #near_sdk::assert_one_yocto();
                let sender_id = #near_sdk::env::predecessor_account_id();
                let amount: u128 = amount.into();

                let transfer = #me::standard::nep171::Nep171Transfer {
                    sender_id: sender_id.clone(),
                    receiver_id: receiver_id.clone(),
                    amount,
                    memo: memo.clone(),
                    msg: None,
                };

                #before_transfer

                let r = #me::standard::nep171::Nep171Controller::transfer_call(
                    self,
                    sender_id.clone(),
                    receiver_id.clone(),
                    amount,
                    memo,
                    msg.clone(),
                    #near_sdk::env::prepaid_gas(),
                );

                #after_transfer

                r
            }

            fn ft_total_supply(&self) -> #near_sdk::json_types::U128 {
                <Self as #me::standard::nep171::Nep171Controller>::total_supply().into()
            }

            fn ft_balance_of(&self, account_id: #near_sdk::AccountId) -> #near_sdk::json_types::U128 {
                <Self as #me::standard::nep171::Nep171Controller>::balance_of(&account_id).into()
            }
        }

        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep171::Nep171Resolver for #ident #ty #wher {
            #[private]
            fn ft_resolve_transfer(
                &mut self,
                sender_id: #near_sdk::AccountId,
                receiver_id: #near_sdk::AccountId,
                amount: #near_sdk::json_types::U128,
            ) -> #near_sdk::json_types::U128 {
                #me::standard::nep171::Nep171Controller::resolve_transfer(
                    self,
                    sender_id,
                    receiver_id,
                    amount.into(),
                ).into()
            }
        }
    })
}