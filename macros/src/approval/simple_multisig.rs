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

        impl #imp ::near_contract_tools::approval::simple_multisig::AccountAuthorizer for #ident #ty #wher {
            type AuthorizationError = ::near_contract_tools::approval::simple_multisig::macro_types::MissingRole;

            fn is_account_authorized(account_id: &AccountId) -> Result<(), Self::AuthorizationError> {
                if <#ident as ::near_contract_tools::rbac::Rbac<_>>::has_role(account_id, &#role) {
                    Ok(())
                } else {
                    Err(::near_contract_tools::approval::simple_multisig::macro_types::MissingRole(format!("{:?}", #role)))
                }
            }
        }
    })
}
