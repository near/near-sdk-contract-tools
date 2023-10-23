//! # Hooks
//!
//! Hooks are a way to wrap (inject code before and after) component functions.
//!
//! Most of the time, hooks are used to implement cross-cutting concerns, such as
//! logging, accounting, or integration with other components.
//!
//! ## Example
//!
//! ```
//! use near_sdk::{env, log, near_bindgen};
//! use near_sdk_contract_tools::{hook::Hook, standard::nep141::*, Nep141};
//!
//! pub struct MyTransferHook;
//!
//! impl Hook<MyContract, Nep141Transfer> for MyTransferHook {
//!     fn hook<R>(contract: &mut MyContract, transfer: &Nep141Transfer, f: impl FnOnce(&mut MyContract) -> R) -> R {
//!         // Log, check preconditions, save state, etc.
//!         log!("NEP-141 transfer from {} to {} of {} tokens", transfer.sender_id, transfer.receiver_id, transfer.amount);
//!
//!         let storage_usage_before = env::storage_usage();
//!
//!         let r = f(contract); // execute wrapped function
//!
//!         let storage_usage_after = env::storage_usage();
//!         log!("Storage delta: {}", storage_usage_after - storage_usage_before);
//!
//!         r
//!     }
//! }
//!
//! #[derive(Nep141)]
//! #[nep141(transfer_hook = "MyTransferHook")]
//! #[near_bindgen]
//! struct MyContract {}
//! ```

/// Generic hook trait for injecting code before and after component functions.
pub trait Hook<C, A = ()> {
    /// Execute a function with hooks.
    fn hook<R>(contract: &mut C, _args: &A, f: impl FnOnce(&mut C) -> R) -> R {
        f(contract)
    }
}

impl<C, A> Hook<C, A> for () {}

impl<C, A, T, U> Hook<C, A> for (T, U)
where
    T: Hook<C, A>,
    U: Hook<C, A>,
{
    fn hook<R>(contract: &mut C, args: &A, f: impl FnOnce(&mut C) -> R) -> R {
        T::hook(contract, args, |contract| U::hook(contract, args, f))
    }
}
