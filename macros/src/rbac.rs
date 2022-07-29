use darling::FromDeriveInput;
use proc_macro::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &str = r#"(b"~r" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(rbac), supports(struct_named))]
pub struct RbacMeta {
    pub storage_key: Option<Expr>,
    pub roles: Expr,

    pub ident: syn::Ident,
}

pub fn expand(meta: RbacMeta) -> Result<TokenStream, darling::Error> {
    let RbacMeta {
        storage_key,
        roles,
        ident,
    } = meta;

    let storage_key =
        storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());

    Ok(TokenStream::from(quote! {
        impl near_contract_tools::rbac::Rbac<#roles> for #ident {
            fn root(&self) -> near_contract_tools::slot::Slot<()> {
                near_contract_tools::slot::Slot::new(#storage_key)
            }
        }
    }))
}
