use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Expr;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(owner), supports(struct_named))]
pub struct OwnerMeta {
    pub storage_key: Option<Expr>,

    pub generics: syn::Generics,
    pub ident: syn::Ident,

    // crates
    #[darling(rename = "crate", default = "crate::default_crate_name")]
    pub me: syn::Path,
    #[darling(default = "crate::default_near_sdk")]
    pub near_sdk: syn::Path,
}

pub fn expand(meta: OwnerMeta) -> Result<TokenStream, darling::Error> {
    let OwnerMeta {
        storage_key,
        ident,
        generics,

        me,
        near_sdk,
    } = meta;

    let (imp, ty, wher) = generics.split_for_impl();

    let root = storage_key.map(|storage_key| {
        quote! {
            fn root() -> #me::slot::Slot<()> {
                #me::slot::Slot::root(#storage_key)
            }
        }
    });

    Ok(quote! {
        impl #imp #me::owner::OwnerInternal for #ident #ty #wher {
            #root
        }

        #[#near_sdk::near_bindgen]
        impl #imp #me::owner::OwnerExternal for #ident #ty #wher {
            fn own_get_owner(&self) -> Option<#near_sdk::AccountId> {
                <Self as #me::owner::OwnerInternal>::slot_owner().read()
            }

            fn own_get_proposed_owner(&self) -> Option<#near_sdk::AccountId> {
                <Self as #me::owner::OwnerInternal>::slot_proposed_owner().read()
            }

            #[payable]
            fn own_renounce_owner(&mut self) {
                #near_sdk::assert_one_yocto();
                #me::owner::Owner::renounce_owner(self);
            }

            #[payable]
            fn own_propose_owner(&mut self, account_id: Option<#near_sdk::AccountId>) {
                #near_sdk::assert_one_yocto();
                #me::owner::Owner::propose_owner(self, account_id);
            }

            #[payable]
            fn own_accept_owner(&mut self) {
                #near_sdk::assert_one_yocto();
                #me::owner::Owner::accept_owner(self);
            }
        }
    })
}
