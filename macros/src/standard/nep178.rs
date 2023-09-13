use std::ops::Not;

use darling::{util::Flag, FromDeriveInput};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep178), supports(struct_named))]
pub struct Nep178Meta {
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

pub fn expand(meta: Nep178Meta) -> Result<TokenStream, darling::Error> {
    let Nep178Meta {
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

    let before_nft_approve = no_hooks.is_present().not().then(|| {
        quote! {
            let hook_state = <Self as #me::standard::nep178::Nep178Hook::<_, _>>::before_nft_approve(&self, &token_id, &account_id);
        }
    });

    let after_nft_approve = no_hooks.is_present().not().then(|| {
        quote! {
            <Self as #me::standard::nep178::Nep178Hook::<_, _>>::after_nft_approve(self, &token_id, &account_id, &approval_id, hook_state);
        }
    });

    let before_nft_revoke = no_hooks.is_present().not().then(|| {
        quote! {
            let hook_state = <Self as #me::standard::nep178::Nep178Hook::<_, _>>::before_nft_revoke(&self, &token_id, &account_id);
        }
    });

    let after_nft_revoke = no_hooks.is_present().not().then(|| {
        quote! {
            <Self as #me::standard::nep178::Nep178Hook::<_, _>>::after_nft_revoke(self, &token_id, &account_id, hook_state);
        }
    });

    let before_nft_revoke_all = no_hooks.is_present().not().then(|| {
        quote! {
            let hook_state = <Self as #me::standard::nep178::Nep178Hook::<_, _>>::before_nft_revoke_all(&self, &token_id);
        }
    });

    let after_nft_revoke_all = no_hooks.is_present().not().then(|| {
        quote! {
            <Self as #me::standard::nep178::Nep178Hook::<_, _>>::after_nft_revoke_all(self, &token_id, hook_state);
        }
    });

    Ok(quote! {
        impl #imp #me::standard::nep178::Nep178ControllerInternal for #ident #ty #wher {
            #root
        }

        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep178::Nep178 for #ident #ty #wher {
            #[payable]
            fn nft_approve(
                &mut self,
                token_id: #me::standard::nep171::TokenId,
                account_id: #near_sdk::AccountId,
                msg: Option<String>,
            ) -> #near_sdk::PromiseOrValue<()> {
                #me::utils::assert_nonzero_deposit();

                let predecessor = #near_sdk::env::predecessor_account_id();

                #before_nft_approve

                let approval_id = #me::standard::nep178::Nep178Controller::approve(
                    self,
                    &token_id,
                    &predecessor,
                    &account_id,
                )
                .unwrap_or_else(|e| #near_sdk::env::panic_str(&e.to_string()));

                #after_nft_approve

                msg.map_or(#near_sdk::PromiseOrValue::Value(()), |msg| {
                    #me::standard::nep178::ext_nep178_receiver::ext(account_id)
                        .nft_on_approve(token_id, predecessor, approval_id, msg)
                        .into()
                })
            }

            #[payable]
            fn nft_revoke(
                &mut self,
                token_id: #me::standard::nep171::TokenId,
                account_id: #near_sdk::AccountId,
            ) {
                #near_sdk::assert_one_yocto();

                let predecessor = #near_sdk::env::predecessor_account_id();

                #before_nft_revoke

                #me::standard::nep178::Nep178Controller::revoke(
                    self,
                    &token_id,
                    &predecessor,
                    &account_id,
                )
                .unwrap_or_else(|e| #near_sdk::env::panic_str(&e.to_string()));

                #after_nft_revoke
            }

            #[payable]
            fn nft_revoke_all(&mut self, token_id: #me::standard::nep171::TokenId) {
                #near_sdk::assert_one_yocto();

                let predecessor = #near_sdk::env::predecessor_account_id();

                #before_nft_revoke_all

                #me::standard::nep178::Nep178Controller::revoke_all(
                    self,
                    &token_id,
                    &predecessor,
                )
                .unwrap_or_else(|e| #near_sdk::env::panic_str(&e.to_string()));

                #after_nft_revoke_all
            }

            fn nft_is_approved(
                &self,
                token_id: #me::standard::nep171::TokenId,
                approved_account_id: #near_sdk::AccountId,
                approval_id: Option<#me::standard::nep178::ApprovalId>,
            ) -> bool {
                match (
                    #me::standard::nep178::Nep178Controller::get_approval_id_for(
                        self,
                        &token_id,
                        &approved_account_id,
                    ),
                    approval_id,
                ) {
                    (Some(saved_approval_id), Some(provided_approval_id)) => saved_approval_id == provided_approval_id,
                    (Some(_), _) => true,
                    _ => false,
                }
            }
        }
    })
}
