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
}

pub fn expand(meta: MigrateMeta) -> Result<TokenStream, darling::Error> {
    let MigrateMeta {
        from,
        to,

        ident,
        generics,
    } = meta;

    let (imp, ty, wh) = generics.split_for_impl();

    let to = to
        .map(|t| t.to_token_stream())
        .unwrap_or_else(|| quote! { Self }.to_token_stream());

    Ok(quote! {
        impl #imp ::near_contract_tools::migrate::MigrateController for #ident #ty #wh {
            type OldSchema = #from;
            type NewSchema = #to;
        }

        #[::near_sdk::near_bindgen]
        impl #imp ::near_contract_tools::migrate::MigrateExternal for #ident #ty #wh {
            #[init(ignore_state)]
            fn migrate(args: Option<String>) -> Self {
                let old_state = <#ident as ::near_contract_tools::migrate::MigrateController>::deserialize_old_schema();

                <#ident as ::near_contract_tools::migrate::MigrateHook>::on_migrate(
                    old_state,
                    args,
                )
            }
        }
    })
}
