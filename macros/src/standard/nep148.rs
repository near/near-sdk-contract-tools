use darling::{FromDeriveInput, ToTokens};
use proc_macro2::TokenStream;
use quote::quote;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(nep148), supports(struct_named))]
pub struct Nep148Meta {
    pub spec: Option<String>,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub reference: Option<String>,
    pub reference_hash: Option<String>,
    pub decimals: u8,

    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

fn optionize<T>(t: Option<T>) -> TokenStream
where
    T: ToTokens,
{
    t.map_or_else(|| quote! { None }, |v| quote! { Some(#v) })
}

pub fn expand(meta: Nep148Meta) -> Result<TokenStream, darling::Error> {
    let Nep148Meta {
        generics,
        ident,
        // fields
        spec,
        name,
        symbol,
        icon,
        reference,
        reference_hash,
        decimals,

        me,
        near_sdk,
    } = meta;

    let spec = spec.map(|s| s.to_token_stream()).unwrap_or_else(|| {
        quote! {
            #me::standard::nep148::FT_METADATA_SPEC
        }
    });

    let icon = optionize(icon);
    let reference = optionize(reference);

    // TODO: Download reference field at compile time and calculate reference_hash automatically
    let reference_hash = optionize(reference_hash.map(|s| {
        let v = format!("{:?}", base64::decode(s).unwrap())
            .parse::<quote::__private::TokenStream>()
            .unwrap();

        quote! { ::std::borrow::Cow::Owned(#near_sdk::json_types::Base64VecU8::from(#v.to_vec())) }
    }));

    let (imp, ty, wher) = generics.split_for_impl();

    Ok(quote! {
        use #me::standard::nep148::Nep148;
        #[#near_sdk::near_bindgen]
        impl #imp #me::standard::nep148::Nep148 for #ident #ty #wher {
            fn ft_metadata(&self) -> #me::standard::nep148::FungibleTokenMetadata<'static> {
                #me::standard::nep148::FungibleTokenMetadata {
                    spec: #spec.into(),
                    name: #name.into(),
                    symbol: #symbol.into(),
                    icon: #icon.map(|s: &str| s.into()),
                    reference: #reference.map(|s: &str| s.into()),
                    reference_hash: #reference_hash,
                    decimals: #decimals,
                }
            }
        }
    })
}
