use darling::{FromDeriveInput, FromMeta};
use once_cell::sync::OnceCell;
use proc_macro2::TokenStream;
use quote::quote;
use regex::Regex;
use syn::Expr;

#[derive(Debug, Clone)]
pub enum HookBody {
    Empty,
    Custom,
    Owner,
    Role(Box<syn::Expr>),
}

impl FromMeta for HookBody {
    fn from_none() -> Option<Self> {
        Some(Self::Custom)
    }

    fn from_string(value: &str) -> darling::Result<Self> {
        static REGEX: OnceCell<Regex> = OnceCell::new();

        if value == "empty" {
            Ok(HookBody::Empty)
        } else if value == "owner" {
            Ok(HookBody::Owner)
        } else {
            let r = REGEX.get_or_init(|| Regex::new(r"^role\((.+)\)$").unwrap());
            r.captures(value)
                .and_then(|c| c.get(1))
                .and_then(|s| syn::parse_str::<Expr>(s.as_str()).ok())
                .map(|e| HookBody::Role(Box::new(e)))
                .ok_or_else(|| {
                    darling::Error::custom(format!(
                        r#"Invalid value "{value}", expected "empty", "owner", or "role(...)""#,
                    ))
                })
        }
    }
}

#[derive(Debug, Clone)]
pub enum Serializer {
    Borsh,
    JsonBase64,
}

impl FromMeta for Serializer {
    fn from_string(value: &str) -> darling::Result<Self> {
        match value {
            "borsh" => Ok(Self::Borsh),
            "jsonbase64" => Ok(Self::JsonBase64),
            _ => Err(darling::Error::custom(format!(
                r#"Invalid value "{value}", expected "borsh" or "jsonbase64""#
            ))),
        }
    }
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(upgrade), supports(struct_named))]
pub struct UpgradeMeta {
    pub hook: HookBody,
    pub serializer: Option<Serializer>,
    pub migrate_method_name: Option<String>,
    pub migrate_method_args: Option<Expr>,
    pub migrate_minimum_gas: Option<Expr>,

    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: UpgradeMeta) -> Result<TokenStream, darling::Error> {
    let UpgradeMeta {
        hook,
        serializer,
        migrate_method_name,
        migrate_method_args,
        migrate_minimum_gas,

        ident,
        generics,

        me,
        near_sdk,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    // Defaults are defined in main crate.
    // I don't think these defaults can be easily defined using
    // #[darling(default = "...")] because they are different types.
    let migrate_method_name = migrate_method_name
        .map(|e| quote! { #e })
        .unwrap_or_else(|| quote! { #me::upgrade::DEFAULT_POST_UPGRADE_METHOD_NAME });
    let migrate_method_args = migrate_method_args
        .map(|e| quote! { #e })
        .unwrap_or_else(|| quote! { #me::upgrade::DEFAULT_POST_UPGRADE_METHOD_ARGS });
    let migrate_minimum_gas = migrate_minimum_gas
        .map(|e| quote! { #e })
        .unwrap_or_else(|| quote! { #me::upgrade::DEFAULT_POST_UPGRADE_MINIMUM_GAS });

    let hook_implementation = match &hook {
        // Should we generate an UpgradeHook implementation with body?
        HookBody::Empty => Some(quote! {}), // empty implementation
        HookBody::Custom => None,           // user-provided implementation
        HookBody::Owner => Some(quote! {
            <Self as #me::owner::Owner>::require_owner();
        }),
        HookBody::Role(role) => Some(quote! {
            #me::rbac::Rbac::require_role(self, &#role);
        }),
    }
    .map(|body| {
        // Interpolate body if implementation is to be generated.
        quote! {
            impl #imp #me::upgrade::serialized::UpgradeHook for #ident #ty #wher {
                fn on_upgrade(&self) {
                    #body
                }
            }
        }
    });

    let (serializer_attribute, code_type, code_conversion) =
        match serializer.unwrap_or(Serializer::JsonBase64) {
            Serializer::Borsh => (
                quote! { #[serializer(borsh)] },
                quote! { Vec<u8> },
                quote! {},
            ),
            Serializer::JsonBase64 => (
                quote! {},
                quote! { #near_sdk::json_types::Base64VecU8 },
                quote! { let code: Vec<u8> = code.into(); },
            ),
        };

    Ok(quote! {
        #[#near_sdk::near_bindgen]
        impl #imp #ident #ty #wher {
            pub fn upgrade(&mut self, #serializer_attribute code: #code_type) {
                #me::upgrade::serialized::UpgradeHook::on_upgrade(self);
                #code_conversion
                #me::upgrade::serialized::upgrade(
                    code,
                    #me::upgrade::PostUpgrade {
                        method: #migrate_method_name.to_string(),
                        args: #migrate_method_args,
                        minimum_gas: #migrate_minimum_gas,
                    },
                );
            }
        }

        #hook_implementation
    })
}
