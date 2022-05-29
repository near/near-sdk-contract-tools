use heck::{
    ToKebabCase, ToLowerCamelCase, ToShoutyKebabCase, ToShoutySnakeCase, ToSnakeCase, ToTitleCase,
    ToUpperCamelCase,
};
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DataEnum, DeriveInput, Fields, Lit, Meta, MetaList, MetaNameValue,
    NestedMeta,
};

/// Derives an NEP-297-compatible event emitting implementation of `Event`.
/// 
/// Specify event standard parameters: `#[event(standard = "...", version = "...")]`
///
/// Rename strategy for all variants (default: unchanged): `#[event(rename_all = "<strategy>")]`
/// Options for `<strategy>`:
/// - `UpperCamelCase`
/// - `lowerCamelCase`
/// - `snake_case`
/// - `kebab-case`
/// - `SHOUTY_SNAKE_CASE`
/// - `SHOUTY-KEBAB-CASE`
/// - `Title Case`
#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    expand(input).unwrap_or_else(|e| e.into_compile_error().into())
}

#[allow(clippy::enum_variant_names)]
enum RenameStrategy {
    UpperCamelCase,
    LowerCamelCase,
    SnakeCase,
    KebabCase,
    ShoutySnakeCase,
    TitleCase,
    ShoutyKebabCase,
}

impl RenameStrategy {
    pub fn transform(&self, s: &str) -> String {
        match self {
            RenameStrategy::UpperCamelCase => s.to_upper_camel_case(),
            RenameStrategy::LowerCamelCase => s.to_lower_camel_case(),
            RenameStrategy::SnakeCase => s.to_snake_case(),
            RenameStrategy::KebabCase => s.to_kebab_case(),
            RenameStrategy::ShoutySnakeCase => s.to_shouty_snake_case(),
            RenameStrategy::TitleCase => s.to_title_case(),
            RenameStrategy::ShoutyKebabCase => s.to_shouty_kebab_case(),
        }
    }
}

impl TryFrom<&str> for RenameStrategy {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "UpperCamelCase" => Ok(Self::UpperCamelCase),
            "lowerCamelCase" => Ok(Self::LowerCamelCase),
            "snake_case" => Ok(Self::SnakeCase),
            "kebab-case" => Ok(Self::KebabCase),
            "SHOUTY_SNAKE_CASE" | "SCREAMING_SNAKE_CASE" | "SHOUTING_SNAKE_CASE" => {
                Ok(Self::ShoutySnakeCase)
            }
            "Title Case" => Ok(Self::TitleCase),
            "SHOUTY-KEBAB-CASE" | "SCREAMING-KEBAB-CASE" | "SHOUTING-KEBAB-CASE" => {
                Ok(Self::ShoutyKebabCase)
            }
            _ => Err(()),
        }
    }
}

fn expand(input: DeriveInput) -> Result<TokenStream, syn::Error> {
    // Container attributes
    let event_attr = input.attrs.iter().filter(|a| a.path.is_ident("event"));

    let mut standard = None;
    let mut version = None;
    let mut rename_all = None;

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
                    } else if path.is_ident("rename_all") {
                        if let Ok(r) = RenameStrategy::try_from(lit.value().as_ref()) {
                            rename_all = Some(r);
                        } else {
                            return Err(syn::Error::new_spanned(lit, "Invalid rename strategy"));
                        }
                    } else {
                        return Err(syn::Error::new_spanned(path, "Invalid key"));
                    }
                }
            }
        }
    }

    if standard.is_none() {
        return Err(syn::Error::new_spanned(
            input,
            r#"Event standard must be specified: #[event(standard = "...")]"#,
        ));
    }

    if version.is_none() {
        return Err(syn::Error::new_spanned(
            input,
            r#"Event standard version must be specified: #[event(version = "...")]"#,
        ));
    }

    let name = &input.ident;

    // Variant attributes
    let mut arms = Vec::new();

    match &input.data {
        Data::Enum(DataEnum { variants, .. }) => {
            for v in variants {
                let id = &v.ident;
                let variant_attr = v
                    .attrs
                    .iter()
                    .find(|a| a.path.is_ident("event"))
                    .map(|a| a.parse_meta());

                let id_str = id.to_string();
                let rename = match variant_attr {
                    Some(Ok(Meta::NameValue(MetaNameValue {
                        lit: Lit::Str(lit), ..
                    }))) => lit.value(),
                    Some(Ok(Meta::List(MetaList { nested, .. }))) => {
                        let mut strategy = None;

                        for p in nested.iter() {
                            match p {
                                NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                                    path,
                                    lit: Lit::Str(lit_str),
                                    ..
                                })) => {
                                    if path.is_ident("rename") {
                                        strategy = Some(
                                            RenameStrategy::try_from(lit_str.value().as_ref())
                                                .map_err(|_| {
                                                    syn::Error::new_spanned(
                                                        lit_str,
                                                        "Invalid rename strategy",
                                                    )
                                                })?,
                                        );
                                    } else {
                                        return Err(syn::Error::new_spanned(path, "Unknown key"));
                                    }
                                }
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        p,
                                        "Unknown attribute format",
                                    ));
                                }
                            }
                        }

                        strategy
                            .as_ref()
                            .or(rename_all.as_ref())
                            .map(|s| s.transform(&id_str))
                            .unwrap_or(id_str)
                    }
                    _ => rename_all
                        .as_ref()
                        .map(|s| s.transform(&id_str))
                        .unwrap_or(id_str),
                };

                arms.push(match v.fields {
                    Fields::Unit => quote! { #name :: #id => #rename , },
                    Fields::Unnamed(..) => {
                        quote! { #name :: #id (..) => #rename , }
                    }
                    Fields::Named(..) => {
                        quote! { #name :: #id {..} => #rename , }
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
            fn standard(&self) -> &'static str {
                #standard
            }

            fn version(&self) -> &'static str {
                #version
            }

            fn event(&self) -> &'static str {
                match self {
                    #(#arms)*
                }
            }
        }
    }))
}
