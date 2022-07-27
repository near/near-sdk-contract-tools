use darling::FromDeriveInput;
use proc_macro::TokenStream;
use syn::Expr;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(fungible_token), supports(struct_named))]
pub struct FungibleTokenMeta {
    pub storage_key: Option<Expr>,

    // NEP-148 fields
    pub spec: Option<String>,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub reference: Option<String>,
    pub reference_hash: Option<String>,
    pub decimals: u8,

    pub generics: syn::Generics,
    pub ident: syn::Ident,
}

pub fn expand(meta: FungibleTokenMeta) -> Result<TokenStream, syn::Error> {
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
    } = meta;

    let expand_nep141 = crate::nep141::expand(crate::nep141::Nep141Meta {
        storage_key,
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

        generics: generics.clone(),
        ident: ident.clone(),
    });

    match (expand_nep141, expand_nep148) {
        (Ok(mut expand_nep141), Ok(expand_nep148)) => {
            // Concatenate token streams
            expand_nep141.extend(expand_nep148);
            Ok(expand_nep141)
        }
        (Err(e), _) | (_, Err(e)) => Err(e),
    }
}
