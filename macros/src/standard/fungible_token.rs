use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

use super::{nep141::{self, Nep141Receiver}, nep148};

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(fungible_token), supports(struct_named))]
pub struct FungibleTokenMeta {
    // NEP-141 fields
    pub storage_key: Option<Expr>,

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
    pub data: darling::ast::Data<(), Nep141Receiver>,
}

pub fn expand(meta: FungibleTokenMeta) -> Result<TokenStream, darling::Error> {
    let FungibleTokenMeta {
        storage_key,

        spec,
        name,
        symbol,
        icon,
        reference,
        reference_hash,
        decimals,

        generics,
        ident,
        data,
    } = meta;

    let expand_nep141 = nep141::expand(nep141::Nep141Meta {
        storage_key,

        generics: generics.clone(),
        ident: ident.clone(),
        data,
    });

    let expand_nep148 = nep148::expand(nep148::Nep148Meta {
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

    let mut e = darling::Error::accumulator();

    let nep141 = e.handle(expand_nep141);
    let nep148 = e.handle(expand_nep148);

    e.finish_with(quote! {
        #nep141
        #nep148
    })
}
