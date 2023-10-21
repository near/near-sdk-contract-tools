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
//! use near_sdk::{env, log, near_bindgen};
//! use near_sdk_contract_tools::{hook::Hook, standard::nep141::*, Nep141};
//!
//! pub struct MyTransferHook;
//!
//! impl Hook<MyContract, Nep141Transfer> for MyTransferHook {
//!     type State = u64;
//!
//!     fn before(contract: &MyContract, transfer: &Nep141Transfer, state: &mut u64) {
//!         // Perform some sort of check before the transfer, e.g.:
//!         // contract.require_registration(&transfer.receiver_id);
//!
//!         // Share state between before and after hooks:
//!         *state = env::storage_usage();
//!     }
//!
//!     fn after(_contract: &mut MyContract, transfer: &Nep141Transfer, state: u64) {
//!         log!("NEP-141 transfer from {} to {} of {} tokens", transfer.sender_id, transfer.receiver_id, transfer.amount);
//!         log!("Storage delta: {}", env::storage_usage() - state);
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
    /// State to be shared between before and after hooks. If no state is needed, use `()`.
    type State: Default;

    /// Before hook. Returns state to be passed to after hook.
    fn before(_contract: &C, _args: &A, _state: &mut Self::State) {}

    /// After hook. Receives state from before hook.
    fn after(_contract: &mut C, _args: &A, _state: Self::State) {}

    /// Execute a function with hooks.
    fn execute<T>(contract: &mut C, args: &A, f: impl FnOnce(&mut C) -> T) -> T {
        let mut state = Self::State::default();
        Self::before(contract, args, &mut state);
        let result = f(contract);
        Self::after(contract, args, state);
        result
    }
}

impl<C, A> Hook<C, A> for () {
    type State = ();
}

impl<C, A, T, U> Hook<C, A> for (T, U)
where
    T: Hook<C, A>,
    U: Hook<C, A>,
{
    type State = (T::State, U::State);

    fn before(contract: &C, args: &A, &mut (ref mut t_state, ref mut u_state): &mut Self::State) {
        T::before(contract, args, t_state);
        U::before(contract, args, u_state);
    }

    fn after(contract: &mut C, args: &A, (t_state, u_state): Self::State) {
        T::after(contract, args, t_state);
        U::after(contract, args, u_state);
    }
}
