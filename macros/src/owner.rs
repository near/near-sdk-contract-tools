use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &str = r#"(b"~o" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(owner), supports(struct_named))]
pub struct OwnerMeta {
    pub storage_key: Option<Expr>,

    pub ident: syn::Ident,
}

pub fn expand(meta: OwnerMeta) -> Result<TokenStream, darling::Error> {
    let OwnerMeta { storage_key, ident } = meta;

    let storage_key =
        storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());

    Ok(quote! {
        impl near_contract_tools::owner::Owner for #ident {
            fn root() -> near_contract_tools::slot::Slot<()> {
                near_contract_tools::slot::Slot::root(#storage_key)
            }
        }

        #[near_sdk::near_bindgen]
        impl near_contract_tools::owner::OwnerExternal for #ident {
            fn own_get_owner(&self) -> Option<near_sdk::AccountId> {
                <Self as near_contract_tools::owner::Owner>::slot_owner().read()
            }

            fn own_get_proposed_owner(&self) -> Option<near_sdk::AccountId> {
                <Self as near_contract_tools::owner::Owner>::slot_proposed_owner().read()
            }

            #[payable]
            fn own_renounce_owner(&mut self) {
                near_sdk::assert_one_yocto();
                self.renounce_owner()
            }

            #[payable]
            fn own_propose_owner(&mut self, account_id: Option<near_sdk::AccountId>) {
                near_sdk::assert_one_yocto();
                self.propose_owner(account_id);
            }

            #[payable]
            fn own_accept_owner(&mut self) {
                near_sdk::assert_one_yocto();
                self.accept_owner();
            }
        }
    })
}
