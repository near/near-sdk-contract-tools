use darling::{FromDeriveInput, ToTokens};
use proc_macro::TokenStream;
use quote::quote;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(migrate), supports(struct_named))]
pub struct MigrateMeta {
    pub from: syn::Type,
    pub to: Option<syn::Type>,
    pub hook: Option<syn::ExprPath>,
    pub args: Option<syn::Type>,

    pub ident: syn::Ident,
    pub generics: syn::Generics,
}

pub fn expand(meta: MigrateMeta) -> Result<TokenStream, syn::Error> {
    let MigrateMeta {
        from,
        to,
        hook,
        args,
        ident,
        generics,
    } = meta;

    let (imp, ty, wh) = generics.split_for_impl();

    let to = to
        .map(|t| t.to_token_stream())
        .unwrap_or_else(|| quote! { Self }.to_token_stream());

    let args_sig = args.as_ref().map(|args| quote! { args: #args });
    let hook_call = hook.map(|hook| quote! { #hook(args) });

    Ok(TokenStream::from(quote! {
        impl #imp near_contract_tools::migrate::MigrateController for #ident #ty #wh {
            type OldSchema = #from;
            type NewSchema = #to;
        }

        #[near_sdk::near_bindgen]
        impl #imp #ident #ty #wh {
            #[private]
            #[init(ignore_state)]
            fn migrate(#args_sig) -> Self {
                #hook_call ;
                <#ident as near_contract_tools::migrate::MigrateController>::convert_state()
            }
        }
    }))
}
