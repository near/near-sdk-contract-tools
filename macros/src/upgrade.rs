use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

const DEFAULT_STORAGE_KEY: &str = r#"(b"~o" as &[u8])"#;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(upgrade), supports(struct_named))]
pub struct UpgradeMeta {
    pub no_default_hook: darling::util::Flag,
    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: UpgradeMeta) -> Result<TokenStream, darling::Error> {
    let UpgradeMeta {
        no_default_hook,
        ident,
        generics,

        me,
        near_sdk,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    let default_hook = (!no_default_hook.is_present()).then_some(quote! {
        impl #imp #me::upgrade::UpgradeHook for #ident #ty #wher {
            fn on_upgrade() {
                <Self as #me::owner::Owner>::require_owner();
            }
        }
    });

    Ok(quote! {
        impl #imp #me::upgrade::Upgrade for #ident #ty #wher {
            #[no_mangle]
            fn upgrade_contract() {
                <Self as #me::upgrade::UpgradeHook>::on_upgrade();
                #me::upgrade::upgrade::<Self>();
            }
        }

        #default_hook
    })
}
