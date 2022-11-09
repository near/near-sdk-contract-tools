//! Contract upgrade functions that work as expected in conjunction with
//! `#[near_bindgen]`.

use near_sdk::{env, GasWeight, Promise};

use super::PostUpgrade;

/// Upgrade lifecycle hooks
pub trait UpgradeHook {
    /// `on_upgrade` should be called when the smart contract is upgraded. If
    /// you use the [`crate::Upgrade`] macro, it will call the hook
    /// automatically for you.
    fn on_upgrade(&self);
}

/// Creates a promise that upgrades the current contract with given code
pub fn upgrade(code: Vec<u8>, post_upgrade: Option<PostUpgrade>) -> Promise {
    match (
        post_upgrade,
        Promise::new(env::current_account_id()).deploy_contract(code),
    ) {
        (Some(post_upgrade), promise) => promise.function_call_weight(
            post_upgrade.method,
            post_upgrade.args,
            0,
            post_upgrade.minimum_gas,
            GasWeight(u64::MAX),
        ),
        (_, promise) => promise,
    }
}

/// Creates a promise that upgrades the current contract with given code and
/// common defaults for the subsequent migration invocation.
pub fn upgrade_with_default_migration(code: Vec<u8>) -> Promise {
    upgrade(code, Some(PostUpgrade::default()))
}
