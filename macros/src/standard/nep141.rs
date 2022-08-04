use darling::{FromDeriveInput, FromField};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &str = r#"(b"~$141" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep141), supports(struct_named))]
pub struct Nep141Meta {
    pub storage_key: Option<Expr>,
    pub generics: syn::Generics,
    pub ident: syn::Ident,
    pub data: darling::ast::Data<(), Nep141Receiver>,
}

#[derive(Debug, FromField)]
pub struct Nep141Receiver {
    pub ident: Option<syn::Ident>,
    pub ty: syn::Type,
}

pub fn expand(meta: Nep141Meta) -> Result<TokenStream, darling::Error> {
    let Nep141Meta {
        storage_key,
        generics,
        ident,
        data,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    let storage_key =
        storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());

    // unwraps are safe because of #[darling(supports(struct_named))]
    let hook = data.take_struct().unwrap().into_iter().find_map(|f| {
        let i = f.ident.unwrap();
        (i.to_string() == "nep141_hook").then(|| f.ty)
    });

    let before_transfer = hook.as_ref().map(|hook| {
        quote! {
            <#hook as near_contract_tools::standard::nep141::Nep141Hook<#ident>>::before_transfer(self, &transfer);
        }
    });

    let after_transfer = hook.as_ref().map(|hook| {
        quote! {
            <#hook as near_contract_tools::standard::nep141::Nep141Hook<#ident>>::after_transfer(self, &transfer);
        }
    });

    Ok(quote! {
        impl #imp near_contract_tools::standard::nep141::Nep141Controller for #ident #ty #wher {
            fn root(&self) -> near_contract_tools::slot::Slot<()> {
                near_contract_tools::slot::Slot::root(#storage_key)
            }
        }

        #[near_sdk::near_bindgen]
        impl #imp near_contract_tools::standard::nep141::Nep141 for #ident #ty #wher {
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
                let amount: u128 = amount.into();

                let transfer = near_contract_tools::standard::nep141::Nep141Transfer {
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
                receiver_id: near_sdk::AccountId,
                amount: near_sdk::json_types::U128,
                memo: Option<String>,
                msg: String,
            ) -> near_sdk::Promise {
                near_sdk::assert_one_yocto();
                let sender_id = near_sdk::env::predecessor_account_id();
                let amount: u128 = amount.into();

                let transfer = near_contract_tools::standard::nep141::Nep141Transfer {
                    sender_id: sender_id.clone(),
                    receiver_id: receiver_id.clone(),
                    amount,
                    memo: memo.clone(),
                    msg: None,
                };

                #before_transfer

                let r = near_contract_tools::standard::nep141::Nep141Controller::transfer_call(
                    self,
                    sender_id.clone(),
                    receiver_id.clone(),
                    amount,
                    memo.as_deref(),
                    msg.clone(),
                    near_sdk::env::prepaid_gas(),
                );

                #after_transfer

                r
            }

            fn ft_total_supply(&self) -> near_sdk::json_types::U128 {
                near_contract_tools::standard::nep141::Nep141Controller::total_supply(self).into()
            }

            fn ft_balance_of(&self, account_id: near_sdk::AccountId) -> near_sdk::json_types::U128 {
                near_contract_tools::standard::nep141::Nep141Controller::balance_of(self, &account_id).into()
            }
        }

        #[near_sdk::near_bindgen]
        impl #imp near_contract_tools::standard::nep141::Nep141Resolver for #ident #ty #wher {
            #[private]
            fn ft_resolve_transfer(
                &mut self,
                sender_id: near_sdk::AccountId,
                receiver_id: near_sdk::AccountId,
                amount: near_sdk::json_types::U128,
            ) -> near_sdk::json_types::U128 {
                near_contract_tools::standard::nep141::Nep141Controller::resolve_transfer(self, sender_id, receiver_id, amount.into()).into()
            }
        }
    })
}
