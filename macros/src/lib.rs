use darling::FromDeriveInput;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod event;
mod owner;
mod pausable;
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

/// Creates a managed, lazily-loaded `Owner` implementation for the targeted
/// `#[near_bindgen]` struct.
///
/// The storage key prefix for the fields can be optionally specified (default:
/// `"~o"`) using `#[ownable(storage_key = "<expression>")]`.
/// ```
#[proc_macro_derive(Owner, attributes(owner))]
pub fn derive_owner(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let meta: owner::OwnerMeta = FromDeriveInput::from_derive_input(&input).unwrap();

    owner::expand(meta).unwrap_or_else(|e| e.into_compile_error().into())
}

/// Makes the contract pausable. Provides an external implementation of the
/// `Pausable` trait, and an internal-only implementation of the
/// `PausableController` trait.
#[proc_macro_derive(Pausable, attributes(pausable))]
pub fn derive_pausable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let meta: pausable::PausableMeta = FromDeriveInput::from_derive_input(&input).unwrap();

    pausable::expand(meta).unwrap_or_else(|e| e.into_compile_error().into())
}
