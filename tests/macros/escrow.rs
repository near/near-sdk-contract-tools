use near_sdk::{
    AccountId, BorshStorageKey, NearToken, PanicOnDefault, VMContext, json_types::U64, near,
    test_utils::VMContextBuilder, testing_env,
};
use near_sdk_contract_tools::{
    Escrow,
    escrow::{Escrow, EscrowInternal},
};

const ID: U64 = U64(1);
const IS_NOT_READY: bool = false;

#[derive(BorshStorageKey)]
#[near]
enum StorageKey {
    MyStorageKey,
}

mod ensure_default {
    use super::*;

    // Ensure compilation of default state type.
    #[derive(Escrow, PanicOnDefault)]
    #[escrow(id = "U64")]
    #[near(contract_state)]
    struct StatelessLock {}
}

#[derive(Escrow, PanicOnDefault)]
#[escrow(id = "U64", state = "bool", storage_key = "StorageKey::MyStorageKey")]
#[near(contract_state)]
struct IsReadyLockableContract {}

#[near]
impl IsReadyLockableContract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }
}

fn alice() -> AccountId {
    "alice".parse().unwrap()
}

fn get_context(attached_deposit: NearToken, signer: Option<AccountId>) -> VMContext {
    VMContextBuilder::new()
        .signer_account_id(signer.clone().unwrap_or_else(alice))
        .predecessor_account_id(signer.unwrap_or_else(alice))
        .attached_deposit(attached_deposit)
        .is_view(false)
        .build()
}

#[test]
fn test_can_lock() {
    testing_env!(get_context(NearToken::from_yoctonear(1u128), None));
    let mut contract = IsReadyLockableContract::new();

    contract.lock(&ID, &IS_NOT_READY);
    assert!(contract.get_locked(&ID).is_some());
}

#[test]
#[should_panic(expected = "Already locked")]
fn test_cannot_lock_twice() {
    testing_env!(get_context(NearToken::from_yoctonear(1u128), None));
    let mut contract = IsReadyLockableContract::new();

    contract.lock(&ID, &IS_NOT_READY);
    contract.lock(&ID, &IS_NOT_READY);
}

#[test]
fn test_can_unlock() {
    testing_env!(get_context(NearToken::from_yoctonear(1u128), None));
    let mut contract = IsReadyLockableContract::new();

    let is_ready = true;
    contract.lock(&ID, &is_ready);
    contract.unlock(&ID, |readiness| readiness == &is_ready);

    assert!(contract.get_locked(&ID).is_none());
}

#[test]
#[should_panic(expected = "Unlock handler failed")]
fn test_cannot_unlock_until_ready() {
    testing_env!(get_context(NearToken::from_yoctonear(1u128), None));
    let mut contract = IsReadyLockableContract::new();

    let is_ready = true;
    contract.lock(&ID, &IS_NOT_READY);
    contract.unlock(&ID, |readiness| readiness == &is_ready);

    assert!(contract.get_locked(&ID).is_none());
}
