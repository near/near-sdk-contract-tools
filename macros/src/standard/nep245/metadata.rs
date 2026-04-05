use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep245_metadata), supports(struct_named))]
pub struct Nep245MetadataMeta {
    pub storage_key: Option<Expr>,

    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: Nep245MetadataMeta) -> Result<TokenStream, darling::Error> {
    let Nep245MetadataMeta {
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

    let st_path = quote! { #me::standard::nep245 };
    let mod_path = quote! { #me::standard::nep245::metadata };

    Ok(quote! {
        impl #imp #mod_path::MetadataControllerInternal for #ident #ty #wher {
            #root
        }

        #[#near_sdk::near]
        impl #imp #mod_path::Nep245Metadata for #ident #ty #wher {
            fn mt_metadata_contract(&self) -> #mod_path::ContractMetadata {
                #mod_path::MetadataController::contract_metadata(self)
            }

            fn mt_metadata_token_all(
                &self,
                token_ids: Vec<#st_path::TokenId>,
            ) -> Vec<#mod_path::TokenMetadataAll> {
                token_ids
                    .into_iter()
                    .flat_map(|token_id| {
                        let stored = #mod_path::MetadataController::token_metadata(self, &token_id)?;
                        let base = #mod_path::MetadataController::base_metadata(self, &stored.base_id)?;
                        Some(#mod_path::TokenMetadataAll {
                            base,
                            token: stored.token,
                        })
                    })
                    .collect()
            }

            fn mt_metadata_token_by_token_id(
                &self,
                token_ids: Vec<#st_path::TokenId>,
            ) -> Vec<#mod_path::TokenMetadata> {
                token_ids
                    .into_iter()
                    .flat_map(|token_id| {
                        let stored = #mod_path::MetadataController::token_metadata(self, &token_id)?;
                        Some(stored.token)
                    })
                    .collect()
            }

            fn mt_metadata_base_by_token_id(
                &self,
                token_ids: Vec<#st_path::TokenId>,
            ) -> Vec<#mod_path::BaseMetadata> {
                token_ids
                    .into_iter()
                    .flat_map(|token_id| {
                        let stored = #mod_path::MetadataController::token_metadata(self, &token_id)?;
                        let base = #mod_path::MetadataController::base_metadata(self, &stored.base_id)?;
                        Some(base)
                    })
                    .collect()
            }

            fn mt_metadata_base_by_metadata_id(
                &self,
                base_metadata_ids: Vec<#mod_path::BaseMetadataId>,
            ) -> Vec<#mod_path::BaseMetadata> {
                base_metadata_ids
                    .into_iter()
                    .flat_map(|metadata_id| #mod_path::MetadataController::base_metadata(self, &metadata_id))
                    .collect()
            }
        }
    })
}
