use darling::FromDeriveInput;
use proc_macro::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &str = r#"(b"$141" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(pause), supports(struct_named))]
pub struct Nep141Meta {
    pub storage_key: Option<Expr>,

    pub generics: syn::Generics,
    pub ident: syn::Ident,
}

pub fn expand(meta: Nep141Meta) -> Result<TokenStream, syn::Error> {
    let Nep141Meta {
        storage_key,
        generics,
        ident,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    let storage_key =
        storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());

    Ok(TokenStream::from(quote! {
        impl #imp near_contract_tools::standard::nep141::Nep141Controller for #ident #ty #wher {
            fn root(&self) -> near_contract_tools::slot::Slot<()> {
                near_contract_tools::slot::Slot::root(#storage_key)
            }
        }

        #[near_sdk::near_bindgen]
        impl #imp near_contract_tools::standard::nep141::Nep141External for #ident #ty #wher {
            #[payable]
            fn ft_transfer(
                &mut self,
                receiver_id: near_sdk::AccountId,
                amount: near_sdk::json_types::U128,
                memo: Option<String>,
            ) {
                use near_contract_tools::{
                    event::Event,
                    standard::nep141::{Nep141Controller, Nep141Event},
                };

                near_sdk::assert_one_yocto();
                let sender_id = near_sdk::env::predecessor_account_id();
                let amount_num: u128 = amount.into();

                Nep141Controller::transfer(self, &sender_id, &receiver_id, amount_num, memo.as_deref());
            }

            #[payable]
            fn ft_transfer_call(
                &mut self,
                receiver_id: near_sdk::AccountId,
                amount: near_sdk::json_types::U128,
                memo: Option<String>,
                msg: String,
            ) -> near_sdk::Promise {
                use near_sdk::{assert_one_yocto, require, env};
                use near_contract_tools::{
                    event::Event,
                    standard::nep141::{
                        ext_nep141_receiver,
                        ext_nep141_resolver,
                        Nep141Controller,
                        Nep141Event,
                        GAS_FOR_FT_TRANSFER_CALL,
                        GAS_FOR_RESOLVE_TRANSFER,
                    },
                };

                assert_one_yocto();
                require!(
                    env::prepaid_gas() > GAS_FOR_FT_TRANSFER_CALL,
                    "More gas is required",
                );

                let sender_id = env::predecessor_account_id();
                let amount_num: u128 = amount.into();

                Nep141Controller::transfer(self, &sender_id, &receiver_id, amount_num, memo.as_deref());

                let receiver_gas = env::prepaid_gas()
                    .0
                    .checked_sub(GAS_FOR_FT_TRANSFER_CALL.0)
                    .unwrap_or_else(|| env::panic_str("Prepaid gas overflow"));
                // Initiating receiver's call and the callback
                ext_nep141_receiver::ext(receiver_id.clone())
                    .with_static_gas(receiver_gas.into())
                    .ft_on_transfer(sender_id.clone(), amount.into(), msg)
                    .then(
                        ext_nep141_resolver::ext(env::current_account_id())
                            .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                            .ft_resolve_transfer(sender_id, receiver_id, amount.into()),
                    )
            }

            fn ft_total_supply(&self) -> near_sdk::json_types::U128 {
                near_contract_tools::standard::nep141::Nep141Controller::total_supply(self).into()
            }

            fn ft_balance_of(&self, account_id: near_sdk::AccountId) -> near_sdk::json_types::U128 {
                near_contract_tools::standard::nep141::Nep141Controller::balance_of(self, &account_id).into()
            }
        }
    }))
}
