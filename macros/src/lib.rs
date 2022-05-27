use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DataEnum, DeriveInput, Fields, Lit, Meta, MetaNameValue, NestedMeta,
};

#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    expand(input).unwrap_or_else(|e| e.into_compile_error().into())
}

fn expand(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    // Container attributes
    let event_attr = input.attrs.iter().filter(|a| a.path.is_ident("event"));

    let mut standard = None;
    let mut version = None;

    for attr in event_attr {
        let meta = attr
            .parse_meta()
            .map_err(|e| syn::Error::new_spanned(attr, e))?;

        if let Meta::List(list) = meta {
            for value in list.nested.iter() {
                if let NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                    path,
                    lit: Lit::Str(lit),
                    ..
                })) = value
                {
                    if path.is_ident("standard") {
                        standard = Some(lit.value());
                    } else if path.is_ident("version") {
                        version = Some(lit.value());
                    }
                }
            }
        }
    }

    if standard.is_none() {
        return Err(syn::Error::new_spanned(
            input,
            "must specify event standard",
        ));
    }

    if version.is_none() {
        return Err(syn::Error::new_spanned(
            input,
            "must specify event standard version",
        ));
    }

    let name = &input.ident;

    // Variant attributes
    let mut arms = Vec::new();

    match &input.data {
        Data::Enum(DataEnum { variants, .. }) => {
            for v in variants {
                let id = &v.ident;
                let rename = match v
                    .attrs
                    .iter()
                    .find(|a| a.path.is_ident("event"))
                    .map(|a| a.parse_meta().ok())
                    .flatten()
                {
                    Some(Meta::NameValue(MetaNameValue {
                        lit: Lit::Str(lit), ..
                    })) => lit.value(),
                    _ => id.to_string(),
                };

                arms.push(match v.fields {
                    Fields::Unit => quote! { #name :: #id => String::from( #rename ) , },
                    Fields::Unnamed(..) => {
                        quote! { #name :: #id (..) => String::from( #rename ) , }
                    }
                    Fields::Named(..) => {
                        quote! { #name :: #id {..} => String::from( #rename ) , }
                    }
                });
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(input, "unsupported structure"));
        }
    }

    Ok(TokenStream::from(quote! {
        impl near_contract_tools::event::EventMetadata for #name {
            fn standard(&self) -> String {
                String::from(#standard)
            }

            fn version(&self) -> String {
                String::from(#version)
            }

            fn event(&self) -> String {
                match self {
                    #(#arms)*
                }
            }
        }
    }))
}
