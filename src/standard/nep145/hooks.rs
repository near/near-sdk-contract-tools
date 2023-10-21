//! Hooks to integrate NEP-145 with other standards.

use near_sdk::env;

use crate::hook::Hook;

use super::Nep145Controller;

/// Hook to perform storage accounting before and after a storage write.
pub struct PredecessorStorageAccountingHook(u64);

impl<C: Nep145Controller, A> Hook<C, A> for PredecessorStorageAccountingHook {
    fn before(contract: &C, _args: &A) -> Self {
        contract
            .get_storage_balance(&env::predecessor_account_id())
            .unwrap_or_else(|e| env::panic_str(&e.to_string()));
        Self(env::storage_usage())
    }

    fn after(contract: &mut C, _args: &A, Self(storage_usage_start): Self) {
        contract
            .storage_accounting(&env::predecessor_account_id(), storage_usage_start)
            .unwrap_or_else(|e| env::panic_str(&format!("Storage accounting error: {}", e)));
    }
}
