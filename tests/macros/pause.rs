compat_use_borsh!(BorshSerialize);
use near_sdk::{near_bindgen, BorshStorageKey};
use near_sdk_contract_tools::{
    compat_derive_storage_key, compat_use_borsh,
    pause::{Pause, PauseExternal},
    Pause,
};

compat_derive_storage_key! {
    enum StorageKey {
        Pause,
    }
}

mod implicit_key {
    use super::*;

    #[derive(Pause)]
    #[near_bindgen]
    struct ContractImplicitKey {}
}

#[derive(Pause)]
#[pause(storage_key = "StorageKey::Pause")]
#[near_bindgen]
struct Contract {
    pub value: u32,
}

#[near_bindgen]
impl Contract {
    pub fn only_when_unpaused(&mut self, value: u32) {
        Self::require_unpaused();

        self.value = value;
    }

    pub fn only_when_paused(&mut self, value: u32) {
        Self::require_paused();

        self.value = value;
    }

    pub fn get_value(&self) -> u32 {
        self.value
    }
}

#[test]
fn derive_pause() {
    let mut contract = Contract { value: 0 };

    assert!(
        !contract.paus_is_paused(),
        "Initial state should be unpaused",
    );

    Contract::require_unpaused();

    contract.pause();

    assert!(contract.paus_is_paused(), "Pausing the contract works");

    Contract::require_paused();

    contract.unpause();

    assert!(!contract.paus_is_paused(), "Unpausing the contract works");

    Contract::require_unpaused();
}

#[test]
fn derive_pause_methods() {
    let mut contract = Contract { value: 0 };

    contract.only_when_unpaused(5);

    assert_eq!(contract.get_value(), 5);

    contract.pause();

    contract.only_when_paused(10);

    assert_eq!(contract.get_value(), 10);
}

#[test]
#[should_panic(expected = "Disallowed while contract is unpaused")]
fn derive_pause_methods_fail_unpaused() {
    let mut contract = Contract { value: 0 };

    contract.only_when_paused(5);
}

#[test]
#[should_panic(expected = "Disallowed while contract is paused")]
fn derive_pause_methods_fail_paused() {
    let mut contract = Contract { value: 0 };

    contract.pause();

    contract.only_when_unpaused(5);
}
