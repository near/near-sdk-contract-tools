use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

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

    let root = storage_key.map(|storage_key| {
        quote! {
            fn root() -> #me::slot::Slot<()> {
                #me::slot::Slot::root(#storage_key)
            }
        }
    });

    Ok(quote! {
        impl #imp #me::approval::ApprovalManager<
                #action,
                #me::approval::simple_multisig::ApprovalState,
                #me::approval::simple_multisig::Configuration<Self>,
            > for #ident #ty #wher {
            #root
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
                    Err(#me::approval::simple_multisig::macro_types::MissingRole(#role))
                }
            }
        }
    })
}
