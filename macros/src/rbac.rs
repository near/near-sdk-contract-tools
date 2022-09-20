use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &str = r#"(b"~r" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(rbac), supports(struct_named))]
pub struct RbacMeta {
    pub storage_key: Option<Expr>,
    pub roles: Expr,

    // darling
    pub ident: syn::Ident,
    pub generics: syn::Generics,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
}

pub fn expand(meta: RbacMeta) -> Result<TokenStream, darling::Error> {
    let RbacMeta {
        storage_key,
        roles,

        ident,
        generics,

        me,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    let storage_key =
        storage_key.unwrap_or_else(|| syn::parse_str::<Expr>(DEFAULT_STORAGE_KEY).unwrap());

    Ok(quote! {
        impl #imp #me::rbac::Rbac for #ident #ty #wher {
            type Role = #roles;

            fn root() -> #me::slot::Slot<()> {
                #me::slot::Slot::new(#storage_key)
            }
        }
    })
}
