use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &str = r#"(b"~sm" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(simple_multisig), supports(struct_named))]
pub struct SimpleMultisigMeta {
    pub storage_key: Option<Expr>,
    pub action: Expr,
    pub role: Expr,

    pub generics: syn::Generics,
    pub ident: syn::Ident,
}

pub fn expand(meta: SimpleMultisigMeta) -> Result<TokenStream, darling::Error> {
    let SimpleMultisigMeta {
        storage_key,
        action,
        role,
        generics,
        ident,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    let storage_key =
        storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());

    Ok(quote! {
        impl #imp ::near_contract_tools::approval::ApprovalManager<
                #action,
                ::near_contract_tools::approval::simple_multisig::ApprovalState,
                ::near_contract_tools::approval::simple_multisig::Configuration<Self>,
            > for #ident #ty #wher {
            fn root() -> ::near_contract_tools::slot::Slot<()> {
                ::near_contract_tools::slot::Slot::root(#storage_key)
            }
        }

        // TODO: This pollutes the global namespace. Is there some better strategy?
        #[derive(Debug, PartialEq)]
        pub enum AccountApproverError {
            UnauthorizedAccount,
        }

        impl ::std::fmt::Display for AccountApproverError {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                ::std::write!(f, "Unauthorized account")
            }
        }

        impl #imp ::near_contract_tools::approval::simple_multisig::AccountApprover for #ident #ty #wher {
            type Error = AccountApproverError;

            fn approve_account(account_id: &::near_sdk::AccountId) -> Result<(), AccountApproverError> {
                if <#ident as ::near_contract_tools::rbac::Rbac<_>>::has_role(account_id, &#role) {
                    Ok(())
                } else {
                    Err(AccountApproverError::UnauthorizedAccount)
                }
            }
        }
    })
}
