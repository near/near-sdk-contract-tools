//! Contract upgrade functions that work as expected in conjunction with
//! `#[near_bindgen]`.

use near_sdk::{env, Gas, GasWeight, Promise};

use super::{
    DEFAULT_MIGRATE_METHOD_ARGS, DEFAULT_MIGRATE_METHOD_NAME, DEFAULT_MIGRATE_MINIMUM_GAS,
};

/// Upgrade lifecycle hooks
pub trait UpgradeHook {
    /// `on_upgrade` should be called when the smart contract is upgraded. If
    /// you use the [`crate::Upgrade`] macro, it will call the hook
    /// automatically for you.
    fn on_upgrade(&self);
}

/// Creates a promise that upgrades the current contract with given code
pub fn upgrade(
    code: Vec<u8>,
    migrate_method_name: String,
    migrate_method_args: Vec<u8>,
    minimum_gas: Gas,
) -> Promise {
    Promise::new(env::current_account_id())
        .deploy_contract(code)
        .function_call_weight(
            migrate_method_name,
            migrate_method_args,
            0,
            minimum_gas,
            GasWeight(u64::MAX),
        )
}

/// Creates a promise that upgrades the current contract with given code and
/// common defaults for the subsequent migration invocation.
pub fn upgrade_with_default_migration(code: Vec<u8>) -> Promise {
    upgrade(
        code,
        DEFAULT_MIGRATE_METHOD_NAME.to_string(),
        DEFAULT_MIGRATE_METHOD_ARGS,
        DEFAULT_MIGRATE_MINIMUM_GAS,
    )
}
