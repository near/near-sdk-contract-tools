compat_use_borsh!();
use near_sdk::{
    json_types::U64, near_bindgen, test_utils::VMContextBuilder, testing_env, AccountId,
    BorshStorageKey, VMContext,
};
use near_sdk_contract_tools::escrow::{Escrow, EscrowInternal};
use near_sdk_contract_tools::{
    compat_derive_storage_key, compat_use_borsh, Escrow, COMPAT_ONE_YOCTONEAR,
};

const ID: U64 = U64(1);
const IS_NOT_READY: bool = false;

compat_derive_storage_key! {
    enum StorageKey {
        MyStorageKey,
    }
}

mod ensure_default {
    use super::*;

    // Ensure compilation of default state type.
    #[derive(Escrow)]
    #[escrow(id = "U64")]
    #[near_bindgen]
    struct StatelessLock {}
}

#[derive(Escrow)]
#[escrow(id = "U64", state = "bool", storage_key = "StorageKey::MyStorageKey")]
#[near_bindgen]
struct IsReadyLockableContract {}

#[near_bindgen]
impl IsReadyLockableContract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }
}

fn alice() -> AccountId {
    "alice".parse().unwrap()
}

fn get_context(
    attached_deposit: near_sdk_contract_tools::CompatNearToken,
    signer: Option<AccountId>,
) -> VMContext {
    VMContextBuilder::new()
        .signer_account_id(signer.clone().unwrap_or_else(alice))
        .predecessor_account_id(signer.unwrap_or_else(alice))
        .attached_deposit(attached_deposit)
        .is_view(false)
        .build()
}

#[test]
fn test_can_lock() {
    testing_env!(get_context(*COMPAT_ONE_YOCTONEAR, None));
    let mut contract = IsReadyLockableContract::new();

    contract.lock(&ID, &IS_NOT_READY);
    assert!(contract.get_locked(&ID).is_some());
}

#[test]
#[should_panic(expected = "Already locked")]
fn test_cannot_lock_twice() {
    testing_env!(get_context(*COMPAT_ONE_YOCTONEAR, None));
    let mut contract = IsReadyLockableContract::new();

    contract.lock(&ID, &IS_NOT_READY);
    contract.lock(&ID, &IS_NOT_READY);
}

#[test]
fn test_can_unlock() {
    testing_env!(get_context(*COMPAT_ONE_YOCTONEAR, None));
    let mut contract = IsReadyLockableContract::new();

    let is_ready = true;
    contract.lock(&ID, &is_ready);
    contract.unlock(&ID, |readiness| readiness == &is_ready);

    assert!(contract.get_locked(&ID).is_none());
}

#[test]
#[should_panic(expected = "Unlock handler failed")]
fn test_cannot_unlock_until_ready() {
    testing_env!(get_context(*COMPAT_ONE_YOCTONEAR, None));
    let mut contract = IsReadyLockableContract::new();

    let is_ready = true;
    contract.lock(&ID, &IS_NOT_READY);
    contract.unlock(&ID, |readiness| readiness == &is_ready);

    assert!(contract.get_locked(&ID).is_none());
}
