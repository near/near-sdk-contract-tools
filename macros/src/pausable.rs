use darling::FromDeriveInput;
use proc_macro::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &'static str = r#"(b"~p" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(pausable), supports(struct_named))]
pub struct PausableMeta {
    pub storage_key: Option<Expr>,

    pub ident: syn::Ident,
}

pub fn expand(meta: PausableMeta) -> Result<TokenStream, syn::Error> {
    let PausableMeta { storage_key, ident } = meta;

    let storage_key = {
        let key =
            storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());
        quote! { &near_sdk::IntoStorageKey::into_storage_key(#key) }
    };

    Ok(TokenStream::from(quote! {
        impl near_contract_tools::pausable::PausableController for #ident {
            fn set_is_paused(&self, is_paused: bool) {
                if is_paused {
                    near_sdk::env::storage_write(#storage_key, &[]);
                } else {
                    near_sdk::env::storage_remove(#storage_key);
                }
            }

            fn is_paused(&self) -> bool {
                near_sdk::env::storage_has_key(#storage_key)
            }
        }

        #[near_sdk::near_bindgen]
        impl near_contract_tools::pausable::Pausable for #ident {
            fn paus_is_paused(&self) -> bool {
                <#ident as near_contract_tools::pausable::PausableController>::is_paused(self)
            }
        }
    }))
}
