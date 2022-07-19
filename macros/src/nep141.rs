use darling::FromDeriveInput;
use proc_macro::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &str = r#"(b"$141" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(pause), supports(struct_named))]
pub struct Nep141Meta {
    pub storage_key: Option<Expr>,

    pub ident: syn::Ident,
}

pub fn expand(meta: Nep141Meta) -> Result<TokenStream, syn::Error> {
    let Nep141Meta { storage_key, ident } = meta;

    let storage_key =
        storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());

    Ok(TokenStream::from(quote! {
        impl near_contract_tools::standard::nep141::Nep141Controller for #ident {
            fn root(&self) -> near_contract_tools::slot::Slot<()> {
                near_contract_tools::slot::Slot::root(#storage_key)
            }
        }

        #[near_sdk::near_bindgen]
        impl near_contract_tools::standard::nep141::Nep141External for #ident {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> Promise;
    fn ft_total_supply(&self) -> U128;
    fn ft_balance_of(&self, account_id: AccountId) -> U128;
        }
    }))
}
