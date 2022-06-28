use darling::FromDeriveInput;
use proc_macro::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &'static str = r#"(b"~o" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(ownable), supports(struct_named))]
pub struct OwnableMeta {
    pub storage_key: Option<Expr>,

    pub ident: syn::Ident,
}

pub fn expand(meta: OwnableMeta) -> Result<TokenStream, syn::Error> {
    let OwnableMeta { storage_key, ident } = meta;

    let storage_key =
        storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());

    let get_ownership = quote! {
        <#ident as near_contract_tools::ownership::OwnershipController>::get_ownership(self)
    };

    Ok(TokenStream::from(quote! {
        impl near_contract_tools::ownership::OwnershipController for #ident {
            fn init_owner(&self, owner_id: near_sdk::AccountId) -> near_contract_tools::ownership::Ownership {
                let storage_key = near_sdk::IntoStorageKey::into_storage_key(#storage_key);

                near_sdk::require!(
                    !near_sdk::env::storage_has_key(&storage_key),
                    "Ownership already initialized",
                );

                let ownership = near_contract_tools::ownership::Ownership::new(#storage_key, owner_id);

                near_sdk::env::storage_write(
                    &storage_key,
                    &near_sdk::borsh::BorshSerialize::try_to_vec(&ownership).unwrap()
                );

                ownership
            }

            fn get_ownership(&self) -> near_contract_tools::ownership::Ownership {
                (near_sdk::borsh::BorshDeserialize::deserialize(
                    &mut (&near_sdk::env::storage_read(
                        &near_sdk::IntoStorageKey::into_storage_key(
                            #storage_key
                        )
                    ).unwrap() as &[u8])
                ) as Result<near_contract_tools::ownership::Ownership, _>).unwrap()
            }
        }

        #[near_sdk::near_bindgen]
        impl near_contract_tools::ownership::Ownable for #ident {
            fn own_get_owner(&self) -> Option<near_sdk::AccountId> {
                #get_ownership.owner
            }

            fn own_get_proposed_owner(&self) -> Option<near_sdk::AccountId> {
                #get_ownership.proposed_owner.get()
            }

            #[payable]
            fn own_renounce_owner(&mut self) {
                near_sdk::assert_one_yocto();
                #get_ownership.renounce_owner()
            }

            #[payable]
            fn own_propose_owner(&mut self, account_id: Option<near_sdk::AccountId>) {
                near_sdk::assert_one_yocto();
                #get_ownership.propose_owner(account_id);
            }

            #[payable]
            fn own_accept_owner(&mut self) {
                near_sdk::assert_one_yocto();
                #get_ownership.accept_owner();
            }
        }
    }))
}
