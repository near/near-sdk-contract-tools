//! Make sure that macros are compatible with each other (e.g. fungible token
//! functions respect paused state)

// TODO: This might not be a good idea and better implemented semi-manually by
//  the user in the form of guards (like in the Migrate macro).

use std::str::FromStr;

use darling::FromMeta;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use strum_macros::EnumString;
use syn::{Meta, NestedMeta};

#[derive(EnumString, Debug)]
enum Integration {
    Pause,
}

impl ToTokens for Integration {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let integration = match self {
            Self::Pause => {
                quote! {
                    <Self as near_contract_tools::pause::Pause>::require_unpaused();
                }
            }
        };

        tokens.extend(integration);
    }
}

#[derive(Default, Debug)]
pub struct IntegrationGuard {
    integrations: Vec<Integration>,
}

impl FromMeta for IntegrationGuard {
    fn from_list(items: &[NestedMeta]) -> darling::Result<Self> {
        let mut errors = darling::Error::accumulator();
        let mut new = Self::default();

        for item in items {
            if let NestedMeta::Meta(Meta::Path(ref path)) = *item {
                let ident = path
                    .get_ident()
                    .map(|s| Integration::from_str(&s.to_string()));

                match ident {
                    Some(Ok(i)) => new.integrations.push(i),
                    Some(_) => {
                        errors.push(darling::Error::unknown_field_path(path).with_span(&path))
                    }
                    _ => errors.push(darling::Error::unexpected_type("non-ident").with_span(path)),
                }
            } else {
                errors.push(darling::Error::unsupported_format("non-word").with_span(item));
            }
        }

        errors.finish_with(new)
    }
}

impl ToTokens for IntegrationGuard {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        for i in self.integrations.iter() {
            i.to_tokens(tokens);
        }
    }
}
