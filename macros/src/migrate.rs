use darling::{FromDeriveInput, ToTokens};
use proc_macro::TokenStream;
use quote::quote;

const ERR_ONLY_ONE_CONVERT_FN: &str = "May only specify up to one of `convert` and `convert_args`";

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(migrate), supports(struct_named))]
pub struct MigrateMeta {
    pub from: syn::Type,
    pub to: Option<syn::Type>,
    pub convert: Option<syn::ExprPath>,
    pub convert_with_args: Option<syn::ExprPath>,
    pub allow: syn::Expr,

    pub ident: syn::Ident,
    pub generics: syn::Generics,
}

pub fn expand(meta: MigrateMeta) -> Result<TokenStream, darling::Error> {
    let MigrateMeta {
        from,
        to,
        convert,
        convert_with_args,
        allow,
        ident,
        generics,
    } = meta;

    let (imp, ty, wh) = generics.split_for_impl();

    let to = to
        .map(|t| t.to_token_stream())
        .unwrap_or_else(|| quote! { Self }.to_token_stream());

    let mut e = darling::Error::accumulator();

    if convert.is_some() && convert_with_args.is_some() {
        e.push(darling::Error::custom(ERR_ONLY_ONE_CONVERT_FN).with_span(&convert));
        e.push(darling::Error::custom(ERR_ONLY_ONE_CONVERT_FN).with_span(&convert_with_args));
    }

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

    e.finish_with(TokenStream::from(quote! {
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
            fn migrate(#args_sig) -> Self {
                #allow;

                let old_state = <#ident as near_contract_tools::migrate::MigrateController>::deserialize_old_schema();

                <#ident as near_contract_tools::migrate::MigrateController>::convert(
                    old_state,
                    #args_val,
                )
            }
        }
    }))
}
