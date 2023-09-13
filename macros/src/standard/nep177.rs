use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep177), supports(struct_named))]
pub struct Nep177Meta {
    pub storage_key: Option<Expr>,

    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: Nep177Meta) -> Result<TokenStream, darling::Error> {
    let Nep177Meta {
        storage_key,

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
        impl #imp #me::standard::nep177::Nep177ControllerInternal for #ident #ty #wher {
            #root
        }

        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep177::Nep177 for #ident #ty #wher {
            fn nft_metadata(&self) -> #me::standard::nep177::ContractMetadata {
                #me::standard::nep177::Nep177Controller::contract_metadata(self)
            }
        }
    })
}
