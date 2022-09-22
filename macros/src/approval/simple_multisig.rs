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

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: SimpleMultisigMeta) -> Result<TokenStream, darling::Error> {
    let SimpleMultisigMeta {
        storage_key,
        action,
        role,
        generics,
        ident,
        me,
        near_sdk,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    let storage_key =
        storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());

    Ok(quote! {
        impl #imp #me::approval::ApprovalManager<
                #action,
                #me::approval::simple_multisig::ApprovalState,
                #me::approval::simple_multisig::Configuration<Self>,
            > for #ident #ty #wher {
            fn root() -> #me::slot::Slot<()> {
                #me::slot::Slot::root(#storage_key)
            }
        }

        impl #imp #me::approval::simple_multisig::AccountAuthorizer for #ident #ty #wher {
            type AuthorizationError =
                #me::approval::simple_multisig::macro_types::MissingRole<
                    <#ident as #me::rbac::Rbac>::Role
                >;

            fn is_account_authorized(account_id: &#near_sdk::AccountId) -> Result<(), Self::AuthorizationError> {
                if <#ident as #me::rbac::Rbac>::has_role(account_id, &#role) {
                    Ok(())
                } else {
                    Err(#me::approval::simple_multisig::macro_types::MissingRole { role: #role })
                }
            }
        }
    })
}
