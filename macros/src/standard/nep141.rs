use std::ops::Not;

use darling::{util::Flag, FromDeriveInput};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &str = r#"(b"~$141" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep141), supports(struct_named))]
pub struct Nep141Meta {
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

pub fn expand(meta: Nep141Meta) -> Result<TokenStream, darling::Error> {
    let Nep141Meta {
        storage_key,
        no_hooks,
        generics,
        ident,

        me,
        near_sdk,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    let storage_key =
        storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());

    let before_transfer = no_hooks.is_present().not().then(|| {
        quote! {
            let hook_state = <Self as #me::standard::nep141::Nep141Hook::<_>>::before_transfer(self, &transfer);
        }
    });

    let after_transfer = no_hooks.is_present().not().then(|| {
        quote! {
            <Self as #me::standard::nep141::Nep141Hook::<_>>::after_transfer(self, &transfer, hook_state);
        }
    });

    Ok(quote! {
        use #me::standard::nep141::Nep141Controller;

        impl #imp #me::standard::nep141::Nep141Controller for #ident #ty #wher {
            fn root() -> #me::slot::Slot<()> {
                #me::slot::Slot::root(#storage_key)
            }
        }

        use #me::standard::nep141::Nep141;

        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep141::Nep141 for #ident #ty #wher {
            #[payable]
            fn ft_transfer(
                &mut self,
                receiver_id: #near_sdk::AccountId,
                amount: #near_sdk::json_types::U128,
                memo: Option<String>,
            ) {
                use #me::{
                    standard::{
                        nep141::{Nep141Controller, Nep141Event},
                        nep297::Event,
                    },
                };

                #near_sdk::assert_one_yocto();
                let sender_id = #near_sdk::env::predecessor_account_id();
                let amount: u128 = amount.into();

                let transfer = #me::standard::nep141::Nep141Transfer {
                    sender_id: sender_id.clone(),
                    receiver_id: receiver_id.clone(),
                    amount,
                    memo: memo.clone(),
                    msg: None,
                };

                #before_transfer

                Nep141Controller::transfer(self, &sender_id, &receiver_id, amount, memo.as_deref());

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

                let transfer = #me::standard::nep141::Nep141Transfer {
                    sender_id: sender_id.clone(),
                    receiver_id: receiver_id.clone(),
                    amount,
                    memo: memo.clone(),
                    msg: None,
                };

                #before_transfer

                let r = #me::standard::nep141::Nep141Controller::transfer_call(
                    self,
                    sender_id.clone(),
                    receiver_id.clone(),
                    amount,
                    memo.as_deref(),
                    msg.clone(),
                    #near_sdk::env::prepaid_gas(),
                );

                #after_transfer

                r
            }

            fn ft_total_supply(&self) -> #near_sdk::json_types::U128 {
                <Self as #me::standard::nep141::Nep141Controller>::total_supply().into()
            }

            fn ft_balance_of(&self, account_id: #near_sdk::AccountId) -> #near_sdk::json_types::U128 {
                <Self as #me::standard::nep141::Nep141Controller>::balance_of(&account_id).into()
            }
        }

        use #me::standard::nep141::Nep141Resolver;

        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep141::Nep141Resolver for #ident #ty #wher {
            #[private]
            fn ft_resolve_transfer(
                &mut self,
                sender_id: #near_sdk::AccountId,
                receiver_id: #near_sdk::AccountId,
                amount: #near_sdk::json_types::U128,
            ) -> #near_sdk::json_types::U128 {
                #me::standard::nep141::Nep141Controller::resolve_transfer(self, sender_id, receiver_id, amount.into()).into()
            }
        }
    })
}
