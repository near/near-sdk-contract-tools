use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep148), supports(struct_named))]
pub struct Nep148Meta {
    pub storage_key: Option<Expr>,
    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: Nep148Meta) -> Result<TokenStream, darling::Error> {
    let Nep148Meta {
        storage_key,
        generics,
        ident,

        me,
        near_sdk,
    } = meta;

    let root = storage_key.map(|storage_key| {
        quote! {
            fn root() -> #me::slot::Slot<()> {
                #me::slot::Slot::root(#storage_key)
            }
        }
    });

    let (imp, ty, wher) = generics.split_for_impl();

    Ok(quote! {
        impl #imp #me::standard::nep148::Nep148ControllerInternal for #ident #ty #wher {
            #root
        }

        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep148::Nep148 for #ident #ty #wher {
            fn ft_metadata(&self) -> #me::standard::nep148::FungibleTokenMetadata {
                #me::standard::nep148::Nep148Controller::get_metadata(self)
            }
        }
    })
}
