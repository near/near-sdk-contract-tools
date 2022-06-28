use darling::FromDeriveInput;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod event;
mod ownable;
mod rename;

/// Derives an NEP-297-compatible event emitting implementation of `Event`.
///
/// Specify event standard parameters: `#[event(standard = "...", version = "...")]`
///
/// Rename strategy for all variants (default: unchanged): `#[event(..., rename_all = "<strategy>")]`
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
    let meta: event::EventMeta = FromDeriveInput::from_derive_input(&input).unwrap();

    event::expand(meta).unwrap_or_else(|e| e.into_compile_error().into())
}

/// Creates a managed, lazily-loaded `Ownership` instance for the targeted
/// `#[near_bindgen]` struct. Creates an externally-accessible `Ownable`
/// implementation, as well as an internal-only `OwnershipController`
/// implementation.
/// 
/// The storage key prefix for the `Ownership` struct can be optionally
/// specified (default: `~o`) using `#[ownable(storage_key = "<expression>")]`.
/// ```
#[proc_macro_derive(Ownable, attributes(ownable))]
pub fn derive_ownable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let meta: ownable::OwnableMeta = FromDeriveInput::from_derive_input(&input).unwrap();

    ownable::expand(meta).unwrap_or_else(|e| e.into_compile_error().into())
}
