use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(migrate), supports(struct_named))]
pub struct MigrateMeta {
    pub from: syn::Type,
    pub to: Option<syn::Type>,

    pub ident: syn::Ident,
    pub generics: syn::Generics,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: MigrateMeta) -> Result<TokenStream, darling::Error> {
    let MigrateMeta {
        from,
        to,

        ident,
        generics,

        me,
        near_sdk,
    } = meta;

    let (imp, ty, wh) = generics.split_for_impl();

    let to = to
        .map(|t| t.to_token_stream())
        .unwrap_or_else(|| quote! { Self }.to_token_stream());

    Ok(quote! {
        impl #imp #me::migrate::MigrateController for #ident #ty #wh {
            type OldSchema = #from;
            type NewSchema = #to;
        }

        #[#near_sdk::near_bindgen]
        impl #imp #ident #ty #wh {
            #[init(ignore_state)]
            pub fn migrate() -> Self {
                let old_state = <#ident as #me::migrate::MigrateController>::deserialize_old_schema();
                <#ident as #me::migrate::MigrateHook>::on_migrate(
                    old_state,
                )
            }
        }
    })
}
