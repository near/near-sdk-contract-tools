use darling::{util::Flag, FromDeriveInput};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep145), supports(struct_named))]
pub struct Nep145Meta {
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

pub fn expand(meta: Nep145Meta) -> Result<TokenStream, darling::Error> {
    let Nep145Meta {
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

    let hook = no_hooks
        .is_present()
        .then(|| quote! { () })
        .unwrap_or_else(|| quote! { Self });

    Ok(quote! {
        impl #imp #me::standard::nep145::Nep145ControllerInternal for #ident #ty #wher {
            type Hook = #hook;

            #root
        }

        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep145::Nep145 for #ident #ty #wher {
            #[payable]
            fn storage_deposit(
                &mut self,
                account_id: Option<#near_sdk::AccountId>,
                registration_only: Option<bool>,
            ) -> #me::standard::nep145::StorageBalance {
                use #me::standard::nep145::*;
                use #near_sdk::{env, json_types::U128, Promise};

                let bounds = Nep145Controller::storage_balance_bounds(self);

                let attached = env::attached_deposit();
                let amount = if registration_only.unwrap_or(false) {
                    bounds.min.0
                } else if let Some(U128(max)) = bounds.max {
                    u128::min(max, attached)
                } else {
                    attached
                };
                let refund = attached.checked_sub(amount).unwrap_or_else(|| {
                    env::panic_str(&format!(
                        "Attached deposit {} is less than required {}",
                        attached, amount,
                    ))
                });
                let predecessor = env::predecessor_account_id();

                let storage_balance = Nep145Controller::storage_deposit(
                    self,
                    &account_id.unwrap_or_else(|| predecessor.clone()),
                    U128(amount),
                )
                .unwrap_or_else(|e| env::panic_str(&format!("Storage deposit error: {}", e)));

                if refund > 0 {
                    Promise::new(predecessor).transfer(amount);
                }

                storage_balance
            }

            #[payable]
            fn storage_withdraw(&mut self, amount: Option<#near_sdk::json_types::U128>) -> #me::standard::nep145::StorageBalance {
                use #me::standard::nep145::*;
                use #near_sdk::{env, json_types::U128, Promise};

                near_sdk::assert_one_yocto();

                let predecessor = env::predecessor_account_id();

                let balance = Nep145Controller::storage_balance(self, &predecessor)
                    .unwrap_or_else(|| env::panic_str("Account is not registered"));

                let amount = amount.unwrap_or(balance.available);

                if amount.0 == 0 {
                    return balance;
                }

                let new_balance = Nep145Controller::storage_withdraw(self, &predecessor, amount)
                    .unwrap_or_else(|e| env::panic_str(&format!("Storage withdraw error: {}", e)));

                Promise::new(predecessor).transfer(amount.0);

                new_balance
            }

            fn storage_unregister(&mut self, force: Option<bool>) -> bool {
                use #me::standard::nep145::*;
                use #near_sdk::{env, Promise};

                near_sdk::assert_one_yocto();

                let predecessor = env::predecessor_account_id();

                let refund = if force.unwrap_or(false) {
                    match Nep145Controller::storage_force_unregister(self, &predecessor) {
                        Ok(refund) => refund,
                        Err(error::StorageForceUnregisterError::AccountNotRegistered(_)) => return false,
                    }
                } else {
                    match Nep145Controller::storage_unregister(self, &predecessor) {
                        Ok(refund) => refund,
                        Err(error::StorageUnregisterError::UnregisterWithLockedBalance(e)) => {
                            env::panic_str(&format!(
                                "Attempt to unregister from storage with locked balance: {}", e
                            ));
                        }
                        Err(error::StorageUnregisterError::AccountNotRegistered(_)) => return false,
                    }
                };

                Promise::new(predecessor).transfer(refund.0);
                true
            }

            fn storage_balance_of(&self, account_id: #near_sdk::AccountId) -> Option<#me::standard::nep145::StorageBalance> {
                #me::standard::nep145::Nep145Controller::storage_balance(self, &account_id)
            }

            fn storage_balance_bounds(&self) -> #me::standard::nep145::StorageBalanceBounds {
                #me::standard::nep145::Nep145Controller::storage_balance_bounds(self)
            }
        }
    })
}
