use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(fungible_token), supports(struct_named))]
pub struct FungibleTokenMeta {
    // NEP-141 fields
    pub storage_key: Option<Expr>,
    pub on_transfer: Option<syn::ExprPath>,
    pub on_transfer_plain: Option<syn::ExprPath>,
    pub on_transfer_call: Option<syn::ExprPath>,

    // NEP-148 fields
    pub spec: Option<String>,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub reference: Option<String>,
    pub reference_hash: Option<String>,
    pub decimals: u8,

    // darling
    pub generics: syn::Generics,
    pub ident: syn::Ident,
}

pub fn expand(meta: FungibleTokenMeta) -> Result<TokenStream, darling::Error> {
    let FungibleTokenMeta {
        storage_key,
        on_transfer,
        on_transfer_plain,
        on_transfer_call,

        spec,
        name,
        symbol,
        icon,
        reference,
        reference_hash,
        decimals,

        generics,
        ident,
    } = meta;

    let expand_nep141 = crate::nep141::expand(crate::nep141::Nep141Meta {
        storage_key,
        on_transfer,
        on_transfer_plain,
        on_transfer_call,
        generics: generics.clone(),
        ident: ident.clone(),
    });

    let expand_nep148 = crate::nep148::expand(crate::nep148::Nep148Meta {
        spec,
        name,
        symbol,
        icon,
        reference,
        reference_hash,
        decimals,

        generics,
        ident,
    });

    match (expand_nep141, expand_nep148) {
        (Ok(expand_nep141), Ok(expand_nep148)) => Ok(quote! {
            #expand_nep141
            #expand_nep148
        }),
        (Err(e), _) | (_, Err(e)) => Err(e),
    }
}
