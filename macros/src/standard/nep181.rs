use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep181), supports(struct_named))]
pub struct Nep181Meta {
    pub storage_key: Option<Expr>,

    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: Nep181Meta) -> Result<TokenStream, darling::Error> {
    let Nep181Meta {
        storage_key,

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

    Ok(quote! {
        impl #imp #me::standard::nep181::Nep181ControllerInternal for #ident #ty #wher {
            #root
        }

        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep181::Nep181 for #ident #ty #wher {
            fn nft_total_supply(&self) -> #near_sdk::json_types::U128 {
                #me::standard::nep181::Nep181Controller::total_enumerated_tokens(self)
                    .into()
            }

            fn nft_tokens(
                &self,
                from_index: Option<#near_sdk::json_types::U128>,
                limit: Option<u32>,
            ) -> Vec<Token> {
                use #me::standard::{
                    nep171::Nep171Controller, nep181::Nep181Controller,
                };

                Nep181Controller::with_tokens(self, |tokens| {
                    let from_index = from_index.map_or(0, |i| i.0 as usize);
                    let it = tokens
                        .iter()
                        .skip(from_index)
                        .map(|token_id| Nep171Controller::load_token(self, token_id).unwrap_or_else(|| {
                            #near_sdk::env::panic_str(&format!("Inconsistent state: Token `{}` is in the enumeration set but its metadata could not be loaded.", token_id))
                        }));

                    if let Some(limit) = limit {
                        it.take(limit as usize).collect()
                    } else {
                        it.collect()
                    }
                })
            }

            fn nft_supply_for_owner(&self, account_id: #near_sdk::AccountId) -> #near_sdk::json_types::U128 {
                #me::standard::nep181::Nep181Controller::with_tokens_for_owner(
                    self,
                    &account_id,
                    |tokens| (tokens.len() as u128).into(),
                )
            }

            fn nft_tokens_for_owner(
                &self,
                account_id: #near_sdk::AccountId,
                from_index: Option<#near_sdk::json_types::U128>,
                limit: Option<u32>,
            ) -> Vec<Token> {
                use #me::standard::{
                    nep171::Nep171Controller, nep181::Nep181Controller,
                };

                Nep181Controller::with_tokens_for_owner(self, &account_id, |tokens| {
                    let from_index = from_index.map_or(0, |i| i.0 as usize);
                    let it = tokens
                        .iter()
                        .skip(from_index)
                        .map(|token_id| Nep171Controller::load_token(self, token_id).unwrap_or_else(|| {
                            #near_sdk::env::panic_str(&format!("Inconsistent state: Token `{}` is in the enumeration set but its metadata could not be loaded.", token_id))
                        }));

                    if let Some(limit) = limit {
                        it.take(limit as usize).collect()
                    } else {
                        it.collect()
                    }
                })
            }
        }
    })
}
