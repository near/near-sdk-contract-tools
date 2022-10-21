use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(pause), supports(struct_named))]
pub struct PauseMeta {
    pub storage_key: Option<Expr>,

    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: PauseMeta) -> Result<TokenStream, darling::Error> {
    let PauseMeta {
        storage_key,
        ident,
        generics,

        me,
        near_sdk,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    let root = storage_key.map(|storage_key| {
        quote! {
            fn root() -> #me::slot::Slot<()> {
                #me::slot::Slot::new(#storage_key)
            }
        }
    });

    Ok(quote! {
        impl #imp #me::pause::Pause for #ident #ty #wher {
            #root
        }

        #[#near_sdk::near_bindgen]
        impl #imp #me::pause::PauseExternal for #ident #ty #wher {
            fn paus_is_paused(&self) -> bool {
                <Self as #me::pause::Pause>::is_paused()
            }
        }
    })
}
