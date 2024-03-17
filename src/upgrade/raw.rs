//! Accepts contract deployment as raw binary.
//!
//! This pattern was common in NEAR smart contracts at the time of writing,
//! but is included here mostly for compatibility/legacy reasons. Unless you
//! are really sure you know what you are doing, you should probably be using
//! [`super::serialized`].
//!
//! # Warning
//!
//! Functions in this module are generally _not callable_ from any call tree
//! originating from a function annotated by `#[near_bindgen]`.

use near_sdk::{env, sys};

use super::PostUpgrade;

/// This function performs low-level, `unsafe` interactions with the NEAR VM.
/// This function automatically sets the return value of the function call to
/// the contract deployment &rarr; migrate function call promise, so the
/// contract should not try to return any other values. This also means that
/// this function probably should not be called from a `#[near_bindgen]`
/// context, since the macro may automatically set a different return value.
///
/// # Safety
///
/// Requires that `near_sdk::env::input()` contains the plain, raw bytes of a
/// valid WebAssembly smart contract.
pub unsafe fn upgrade(post_upgrade: PostUpgrade) {
    // Create a promise batch
    let promise_id = sys::promise_batch_create(
        env::current_account_id().as_bytes().len() as u64,
        env::current_account_id().as_bytes().as_ptr() as u64,
    );

    sys::input(0);

    // Deploy the contract code
    sys::promise_batch_action_deploy_contract(promise_id, u64::MAX, 0);

    #[cfg(feature = "near-sdk-4")]
    let gas = post_upgrade.minimum_gas.0;
    #[cfg(feature = "near-sdk-5")]
    let gas = post_upgrade.minimum_gas.as_gas();

    // Call promise to migrate the state.
    // Batched together to fail upgrade if migration fails.
    sys::promise_batch_action_function_call_weight(
        promise_id,
        post_upgrade.method.len() as u64,
        post_upgrade.method.as_ptr() as u64,
        post_upgrade.args.len() as u64,
        post_upgrade.args.as_ptr() as u64,
        0,
        gas,
        u64::MAX,
    );

    sys::promise_return(promise_id);
}
