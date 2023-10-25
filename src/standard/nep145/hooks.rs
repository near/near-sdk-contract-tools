//! Hooks to integrate NEP-145 with other standards.

use near_sdk::env;

use crate::hook::Hook;

use super::Nep145Controller;

/// Hook to perform storage accounting before and after a storage write.
pub struct PredecessorStorageAccountingHook;

impl<C: Nep145Controller, A> Hook<C, A> for PredecessorStorageAccountingHook {
    fn hook<R>(contract: &mut C, _args: &A, f: impl FnOnce(&mut C) -> R) -> R {
        let storage_usage_start = env::storage_usage();
        let predecessor = env::predecessor_account_id();

        contract
            .get_storage_balance(&predecessor)
            .unwrap_or_else(|e| env::panic_str(&e.to_string()));

        let r = f(contract);

        contract
            .storage_accounting(&predecessor, storage_usage_start)
            .unwrap_or_else(|e| env::panic_str(&format!("Storage accounting error: {}", e)));

        r
    }
}
