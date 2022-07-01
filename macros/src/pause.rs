use darling::FromDeriveInput;
use proc_macro::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &str = r#"(b"~p" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(pause), supports(struct_named))]
pub struct PauseMeta {
    pub storage_key: Option<Expr>,

    pub ident: syn::Ident,
}

pub fn expand(meta: PauseMeta) -> Result<TokenStream, syn::Error> {
    let PauseMeta { storage_key, ident } = meta;

    let storage_key =
        storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());

    Ok(TokenStream::from(quote! {
        impl near_contract_tools::pause::PauseStorage for #ident {
            fn slot_paused(&self) -> near_contract_tools::slot::Slot<bool> {
                near_contract_tools::slot::Slot::new(#storage_key)
            }
        }

        #[near_sdk::near_bindgen]
        impl near_contract_tools::pause::Pause for #ident {
            fn paus_is_paused(&self) -> bool {
                self.is_paused()
            }
        }
    }))
}
