//! Hooks to integrate NEP-145 with other components.

use near_sdk::{env, AccountId};

use crate::{
    hook::Hook,
    standard::{
        nep141::{Nep141Burn, Nep141Mint, Nep141Transfer},
        nep171::action::{Nep171Burn, Nep171Mint, Nep171Transfer},
    },
};

use super::Nep145Controller;

fn require_registration(contract: &impl Nep145Controller, account_id: &AccountId) {
    contract
        .get_storage_balance(account_id)
        .unwrap_or_else(|e| env::panic_str(&e.to_string()));
}

fn apply_storage_accounting_hook<C: Nep145Controller, R>(
    contract: &mut C,
    account_id: &AccountId,
    f: impl FnOnce(&mut C) -> R,
) -> R {
    let storage_usage_start = env::storage_usage();
    require_registration(contract, account_id);

    let r = f(contract);

    contract
        .storage_accounting(account_id, storage_usage_start)
        .unwrap_or_else(|e| env::panic_str(&format!("Storage accounting error: {}", e)));

    r
}

/// Hook to perform storage accounting before and after a storage write.
pub struct PredecessorStorageAccountingHook;

impl<C: Nep145Controller, A> Hook<C, A> for PredecessorStorageAccountingHook {
    fn hook<R>(contract: &mut C, _args: &A, f: impl FnOnce(&mut C) -> R) -> R {
        apply_storage_accounting_hook(contract, &env::predecessor_account_id(), f)
    }
}

/// NEP-141 support for NEP-145.
pub struct Nep141StorageAccountingHook;

impl<C: Nep145Controller> Hook<C, Nep141Mint<'_>> for Nep141StorageAccountingHook {
    fn hook<R>(contract: &mut C, action: &Nep141Mint<'_>, f: impl FnOnce(&mut C) -> R) -> R {
        apply_storage_accounting_hook(contract, action.receiver_id, f)
    }
}

impl<C: Nep145Controller> Hook<C, Nep141Transfer<'_>> for Nep141StorageAccountingHook {
    fn hook<R>(contract: &mut C, action: &Nep141Transfer<'_>, f: impl FnOnce(&mut C) -> R) -> R {
        apply_storage_accounting_hook(contract, action.receiver_id, f)
    }
}

impl<C: Nep145Controller> Hook<C, Nep141Burn<'_>> for Nep141StorageAccountingHook {
    fn hook<R>(contract: &mut C, _action: &Nep141Burn<'_>, f: impl FnOnce(&mut C) -> R) -> R {
        f(contract)
    }
}

/// NEP-171 support for NEP-145.
pub struct Nep171StorageAccountingHook;

impl<C: Nep145Controller> Hook<C, Nep171Mint<'_>> for Nep171StorageAccountingHook {
    fn hook<R>(contract: &mut C, action: &Nep171Mint<'_>, f: impl FnOnce(&mut C) -> R) -> R {
        apply_storage_accounting_hook(contract, action.receiver_id, f)
    }
}

impl<C: Nep145Controller> Hook<C, Nep171Transfer<'_>> for Nep171StorageAccountingHook {
    fn hook<R>(contract: &mut C, action: &Nep171Transfer<'_>, f: impl FnOnce(&mut C) -> R) -> R {
        apply_storage_accounting_hook(contract, action.receiver_id, f)
    }
}

impl<C: Nep145Controller> Hook<C, Nep171Burn<'_>> for Nep171StorageAccountingHook {
    fn hook<R>(contract: &mut C, _action: &Nep171Burn<'_>, f: impl FnOnce(&mut C) -> R) -> R {
        f(contract)
    }
}
