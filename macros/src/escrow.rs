use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(escrow), supports(struct_named))]
pub struct EscrowMeta {
    pub storage_key: Option<Expr>,
    pub id: Expr,
    pub state: Expr,

    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: EscrowMeta) -> Result<TokenStream, darling::Error> {
    let EscrowMeta {
        storage_key,
        id,
        state,

        ident,
        generics,

        me,
        near_sdk: _near_sdk,
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
        impl #imp #me::escrow::EscrowInternal for #ident #ty #wher {
            type Id = #id;
            type State = #state;

            #root
        }
    })
}
