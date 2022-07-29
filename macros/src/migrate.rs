use darling::{FromDeriveInput, ToTokens};
use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned;

#[derive(Debug, FromDeriveInput)]
#[darling(
    attributes(migrate),
    supports(struct_named),
    and_then = "Self::validate"
)]
pub struct MigrateMeta {
    pub from: syn::Type,
    pub to: Option<syn::Type>,
    pub convert: Option<syn::ExprPath>,
    pub convert_with_args: Option<syn::ExprPath>,
    pub allow: Option<syn::ExprPath>,
    pub allow_if: Option<syn::Expr>,

    pub ident: syn::Ident,
    pub generics: syn::Generics,
}

fn mutually_exclusive<T: Spanned, U: Spanned>(
    e: &mut darling::error::Accumulator,
    a: &Option<T>,
    b: &Option<U>,
    msg: &str,
) {
    if let (Some(a), Some(b)) = (a, b) {
        e.push(darling::Error::custom(msg).with_span(a));
        e.push(darling::Error::custom(msg).with_span(b));
    }
}

impl MigrateMeta {
    fn validate(self) -> darling::Result<Self> {
        let mut e = darling::Error::accumulator();

        mutually_exclusive(
            &mut e,
            &self.convert,
            &self.convert_with_args,
            "`convert` and `convert_with_args` are mutually exclusive",
        );
        mutually_exclusive(
            &mut e,
            &self.allow,
            &self.allow_if,
            "`allow` and `allow_if` are mutually exclusive",
        );

        if self.allow.is_none() && self.allow_if.is_none() {
            e.push(darling::Error::missing_field(
                "One of `allow` or `allow_if` is required",
            ));
        }

        e.finish_with(self)
    }
}

pub fn expand(meta: MigrateMeta) -> Result<TokenStream, darling::Error> {
    let MigrateMeta {
        from,
        to,
        convert,
        convert_with_args,
        allow,
        allow_if,
        ident,
        generics,
    } = meta;

    let (imp, ty, wh) = generics.split_for_impl();

    let to = to
        .map(|t| t.to_token_stream())
        .unwrap_or_else(|| quote! { Self }.to_token_stream());

    let convert_body = convert_with_args
        .as_ref()
        .map(|fn_name| quote! { #fn_name(old_state, args.unwrap()) })
        .or_else(|| convert.map(|fn_name| quote! { #fn_name(old_state) }))
        .unwrap_or_else(|| quote! { <Self::NewSchema as From<Self::OldSchema>>::from(old_state) });

    let args_sig = convert_with_args.as_ref().map(|_| quote! { args: String });
    let args_val = convert_with_args
        .as_ref()
        .map(|_| quote! { Some(args) })
        .unwrap_or_else(|| quote! { None });

    let allow_stmt = allow_if
        .map(|allow_if| {
            quote! {
                near_sdk::require!(
                    #allow_if,
                    "Migration is not allowed",
                )
            }
        })
        .or_else(|| allow.map(|allow| quote! { #allow() }))
        .unwrap(); // Guaranteed because of validate function

    Ok(TokenStream::from(quote! {
        impl #imp near_contract_tools::migrate::MigrateController for #ident #ty #wh {
            type OldSchema = #from;
            type NewSchema = #to;

            fn convert(old_state: Self::OldSchema, args: Option<String>) -> Self::NewSchema {
                #convert_body
            }
        }

        #[near_sdk::near_bindgen]
        impl #imp #ident #ty #wh {
            #[init(ignore_state)]
            pub fn migrate(#args_sig) -> Self {
                #allow_stmt;

                let old_state = <#ident as near_contract_tools::migrate::MigrateController>::deserialize_old_schema();

                <#ident as near_contract_tools::migrate::MigrateController>::convert(
                    old_state,
                    #args_val,
                )
            }
        }
    }))
}
