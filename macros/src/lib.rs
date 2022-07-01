use darling::FromDeriveInput;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod event;
mod owner;
mod pause;
mod rbac;
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
/// `"~o"`) using `#[owner(storage_key = "<expression>")]`.
/// ```
#[proc_macro_derive(Owner, attributes(owner))]
pub fn derive_owner(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let meta: owner::OwnerMeta = FromDeriveInput::from_derive_input(&input).unwrap();

    owner::expand(meta).unwrap_or_else(|e| e.into_compile_error().into())
}

/// Makes a contract pausable. Provides an implementation of the `Pause` trait.
///
/// The storage key prefix for the fields can be optionally specified (default:
/// `"~p"`) using `#[pause(storage_key = "<expression>")]`.
#[proc_macro_derive(Pause, attributes(pause))]
pub fn derive_pause(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let meta: pause::PauseMeta = FromDeriveInput::from_derive_input(&input).unwrap();

    pause::expand(meta).unwrap_or_else(|e| e.into_compile_error().into())
}

/// Adds role-based access control. No external methods are exposed.
///
/// The storage key prefix for the fields can be optionally specified (default:
/// `"~r"`) using `#[pause(storage_key = "<expression>")]`.
#[proc_macro_derive(Rbac, attributes(rbac))]
pub fn derive_rbac(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let meta: rbac::RbacMeta = FromDeriveInput::from_derive_input(&input).unwrap();

    rbac::expand(meta).unwrap_or_else(|e| e.into_compile_error().into())
}
