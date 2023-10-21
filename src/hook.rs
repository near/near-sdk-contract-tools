//! # Hooks
//!
//! Hooks are a way to inject code before and after contract functions.
//!
//! Most of the time, hooks are used to implement cross-cutting concerns, such as
//! logging, accounting, or integration with other standards.
//!
//! ## Example
//!
//! ```
//! use near_sdk::{log, near_bindgen};
//! use near_sdk_contract_tools::{hook::Hook, standard::nep141::*, Nep141};
//!
//! pub struct MyTransferHook;
//!
//! impl Hook<MyContract, Nep141Transfer> for MyTransferHook {
//!     fn before(contract: &MyContract, transfer: &Nep141Transfer) -> Self {
//!         // Perform some sort of check before the transfer, e.g.:
//!         // contract.require_registration(&transfer.receiver_id);
//!         Self
//!     }
//!
//!     fn after(_contract: &mut MyContract, transfer: &Nep141Transfer, _: Self) {
//!         log!("NEP-141 transfer from {} to {} of {} tokens", transfer.sender_id, transfer.receiver_id, transfer.amount);
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
    /// Before hook. Returns state to be passed to after hook.
    fn before(contract: &C, args: &A) -> Self;

    /// After hook. Receives state from before hook.
    fn after(contract: &mut C, args: &A, state: Self);
}

impl<C, A> Hook<C, A> for () {
    fn before(_contract: &C, _args: &A) {}
    fn after(_contract: &mut C, _args: &A, _: ()) {}
}

impl<C, A, T, U> Hook<C, A> for (T, U)
where
    T: Hook<C, A>,
    U: Hook<C, A>,
{
    fn before(contract: &C, args: &A) -> Self {
        (T::before(contract, args), U::before(contract, args))
    }

    fn after(contract: &mut C, args: &A, (t_state, u_state): Self) {
        T::after(contract, args, t_state);
        U::after(contract, args, u_state);
    }
}
