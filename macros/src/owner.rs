use darling::FromDeriveInput;
use proc_macro::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &str = r#"(b"~o" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(owner), supports(struct_named))]
pub struct OwnerMeta {
    pub storage_key: Option<Expr>,

    pub ident: syn::Ident,
}

pub fn expand(meta: OwnerMeta) -> Result<TokenStream, syn::Error> {
    let OwnerMeta { storage_key, ident } = meta;

    let storage_key =
        storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());

    let root = quote! {
        near_contract_tools::slot::Slot::root(#storage_key)
    };

    Ok(TokenStream::from(quote! {
        impl near_contract_tools::owner::OwnerStorage for #ident {
            fn is_initialized(&self) -> near_contract_tools::slot::Slot<bool> {
                #root.field(b"i")
            }

            fn owner(&self) -> near_contract_tools::slot::Slot<near_sdk::AccountId> {
                #root.field(b"o")
            }

            fn proposed_owner(&self) -> near_contract_tools::slot::Slot<near_sdk::AccountId> {
                #root.field(b"p")
            }
        }

        #[near_sdk::near_bindgen]
        impl near_contract_tools::owner::Owner for #ident {
            fn own_get_owner(&self) -> Option<near_sdk::AccountId> {
                near_contract_tools::owner::OwnerStorage::owner(self).read()
            }

            fn own_get_proposed_owner(&self) -> Option<near_sdk::AccountId> {
                near_contract_tools::owner::OwnerStorage::proposed_owner(self).read()
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
    }))
}
