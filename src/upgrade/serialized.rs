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
pub fn upgrade(code: Vec<u8>, post_upgrade: PostUpgrade) -> Promise {
    Promise::new(env::current_account_id())
        .deploy_contract(code)
        .function_call_weight(
            post_upgrade.method,
            post_upgrade.args,
            compat_yoctonear!(0u128),
            post_upgrade.minimum_gas,
            GasWeight(u64::MAX),
        )
}

/// Creates a promise that upgrades the current contract with given code and
/// common defaults for the subsequent post-upgrade invocation.
pub fn upgrade_default(code: Vec<u8>) -> Promise {
    upgrade(code, PostUpgrade::default())
}
