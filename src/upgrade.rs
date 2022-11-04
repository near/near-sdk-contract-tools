//! Upgrade pattern implements methods to upgrade the contract state and code.
//!
//! Upgrade does not have a default implementation and must be implemented
//! by the user. For a complete example checkout [upgrade_old.rs](https://github.com/NEARFoundation/near-contract-tools/blob/develop/workspaces-tests/src/bin/upgrade_old.rs)
//! in workspace-tests.
//!
//! # Safety
//! Upgrade internally calls [migrate](super::migrate::MigrateExternal)
//! and has the same invariants. The contract state must conform to the old
//! schema otherwise deserializing it will fail and throw an error.
use near_sdk::{env, sys, Gas};

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

/// Upgrade Trait
pub trait Upgrade {
    /// upgrade_contract - Upgrades the code and the state of the contract
    fn upgrade_contract();
}

/// Contracts may implement this trait to inject code into Upgrade functions.
pub trait UpgradeHook {
    /// Executed before a upgrade call is conducted
    fn on_upgrade();
}

/// Naked upgrade function which calls migrate method on the contract
pub fn upgrade<T>(code: Vec<u8>)
where
    T: BorshDeserialize + BorshSerialize,
{
    env::setup_panic_hook();

    const MIGRATE_METHOD_NAME: &[u8; 7] = b"migrate";
    const UPDATE_GAS_LEFTOVER: Gas = Gas(5_000_000_000_000);

    env::log_str("Calling Upgrade ...");

    unsafe {
        // Load code into register 0 result from the input argument if factory call or from promise if callback.
        sys::input(0);
        env::log_str("Creating Promise ...");

        // Create a promise batch to update current contract with code from register 0.
        let promise_id = sys::promise_batch_create(
            env::current_account_id().as_bytes().len() as u64,
            env::current_account_id().as_bytes().as_ptr() as u64,
        );

        env::log_str("Deploying ...");
        env::log_str(&format!(
            "prepaid {:?} , used {:?}",
            env::prepaid_gas(),
            env::used_gas()
        ));

        // Deploy the contract code from register 0.
        sys::promise_batch_action_deploy_contract(
            promise_id,
            code.len() as u64,
            code.as_ptr() as u64,
        );

        env::log_str("Calling migrate ...");
        env::log_str(&format!(
            "prepaid {:?} , used {:?}, {}",
            env::prepaid_gas(),
            env::used_gas(),
            (env::prepaid_gas() - env::used_gas() - UPDATE_GAS_LEFTOVER).0
        ));

        // Call promise to migrate the state.
        // Batched together to fail upgrade if migration fails.
        sys::promise_batch_action_function_call(
            promise_id,
            MIGRATE_METHOD_NAME.len() as u64,
            MIGRATE_METHOD_NAME.as_ptr() as u64,
            0,
            0,
            0,
            (env::prepaid_gas() - env::used_gas() - UPDATE_GAS_LEFTOVER).0,
        );

        env::log_str("after Return ...");

        sys::promise_return(promise_id);

        env::log_str("End of upgrade");
    }
}
