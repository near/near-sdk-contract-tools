use darling::FromDeriveInput;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(ownable), supports(struct_named))]
pub struct OwnableMeta {
    pub storage_key: Option<Box<dyn IntoStorageKey>>,
    pub version: String,

    pub ident: syn::Ident,
    pub data: darling::ast::Data<(), ()>,
}
