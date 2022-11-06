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

use near_sdk::{env, sys, Gas};

use super::{
    DEFAULT_MIGRATE_METHOD_ARGS, DEFAULT_MIGRATE_METHOD_NAME, DEFAULT_MIGRATE_MINIMUM_GAS,
};

/// This function performs low-level, `unsafe` interactions with the NEAR VM.
/// This function automatically sets the return value of the function call to
/// the contract deployment &rarr; migrate function call promise, so the
/// contract should not try to return any other values. This also means that
/// this function probably should not be called from a `#[near_bindgen]`
/// context, since it may automatically set a return value.
///
/// This function is called by this module's other public functions:
/// [`upgrade_from_transaction_input`] and [`upgrade_from_vec`].
///
/// # Safety
///
/// `len` and `ptr` must be valid values to pass into
/// `near_sys::promise_batch_action_deploy_contract` (i.e. pointer to a valid
/// WASM blob or a register descriptor).
pub unsafe fn finish_upgrade(
    len: u64,
    ptr: u64,
    migrate_method_name: &str,
    migrate_method_args: Vec<u8>,
    migrate_minimum_gas: Gas,
) {
    // Create a promise batch
    let promise_id = sys::promise_batch_create(
        env::current_account_id().as_bytes().len() as u64,
        env::current_account_id().as_bytes().as_ptr() as u64,
    );
    // Deploy the contract code
    sys::promise_batch_action_deploy_contract(promise_id, len, ptr);
    // Call promise to migrate the state.
    // Batched together to fail upgrade if migration fails.
    sys::promise_batch_action_function_call_weight(
        promise_id,
        migrate_method_name.len() as u64,
        migrate_method_name.as_ptr() as u64,
        migrate_method_args.len() as u64,
        migrate_method_args.as_ptr() as u64,
        0,
        migrate_minimum_gas.0,
        u64::MAX,
    );

    sys::promise_return(promise_id);
}

/// Upgrades the contract using the raw VM input as the code to deploy and
/// common defaults for the subsequent migration method invocation.
pub fn upgrade_from_transaction_input() {
    unsafe {
        sys::input(0);
        finish_upgrade(
            u64::MAX,
            0,
            DEFAULT_MIGRATE_METHOD_NAME,
            DEFAULT_MIGRATE_METHOD_ARGS,
            DEFAULT_MIGRATE_MINIMUM_GAS,
        );
    }
}

/// Upgrades the contract by deploying the provided code as the new contract
/// and using common defaults for the subsequent migration method invocation.
pub fn upgrade_from_vec(code: Vec<u8>) {
    unsafe {
        finish_upgrade(
            code.len() as u64,
            code.as_ptr() as u64,
            DEFAULT_MIGRATE_METHOD_NAME,
            DEFAULT_MIGRATE_METHOD_ARGS,
            DEFAULT_MIGRATE_MINIMUM_GAS,
        );
    }
}
