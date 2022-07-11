use near_contract_tools::{owner::Owner, pause::Pause, rbac::Rbac, Owner, Pause, Rbac};
use near_sdk::{
    borsh::{self, BorshSerialize},
    near_bindgen,
    test_utils::VMContextBuilder,
    testing_env, AccountId, BorshStorageKey,
};

mod event;
mod owner;
mod pause;

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Owner,
    Pause,
    Rbac,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum Role {
    CanPause,
    CanSetValue,
}

#[derive(Owner, Pause, Rbac)]
#[owner(storage_key = "StorageKey::Owner")]
#[pause(storage_key = "StorageKey::Pause")]
#[rbac(storage_key = "StorageKey::Rbac", roles = "Role")]
#[near_bindgen]
struct Integration {
    pub value: u32,
}

#[near_bindgen]
impl Integration {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        let mut contract = Self { value: 0 };

        Owner::init(&contract, owner_id.clone());
        contract.add_role(&owner_id, &Role::CanSetValue);
        contract.add_role(&owner_id, &Role::CanPause);

        contract
    }

    pub fn add_value_setter(&mut self, account_id: AccountId) {
        self.require_owner();

        self.add_role(&account_id, &Role::CanSetValue);
    }

    pub fn set_value(&mut self, value: u32) {
        self.require_unpaused();
        self.require_role(&Role::CanSetValue);

        self.value = value;
    }

    pub fn pause(&mut self) {
        self.require_role(&Role::CanPause);
        Pause::pause(self);
    }

    pub fn unpause(&mut self) {
        self.require_role(&Role::CanPause);
        Pause::unpause(self);
    }

    pub fn get_value(&self) -> u32 {
        self.value
    }
}

#[test]
fn integration() {
    let owner: AccountId = "owner".parse().unwrap();
    let alice: AccountId = "alice".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let mut c = Integration::new(owner.clone());

    c.set_value(5);

    assert_eq!(c.get_value(), 5);

    c.add_value_setter(alice.clone());

    let context = VMContextBuilder::new()
        .predecessor_account_id(alice.clone())
        .build();

    testing_env!(context);

    c.set_value(15);

    assert_eq!(c.get_value(), 15);

    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);

    Integration::pause(&mut c);
    Integration::unpause(&mut c);

    c.set_value(25);

    assert_eq!(c.get_value(), 25);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn integration_fail_missing_role() {
    let owner: AccountId = "owner".parse().unwrap();
    let alice: AccountId = "alice".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let mut c = Integration::new(owner.clone());

    let context = VMContextBuilder::new()
        .predecessor_account_id(alice.clone())
        .build();

    testing_env!(context);

    c.set_value(15);
}

#[test]
#[should_panic(expected = "Disallowed while contract is paused")]
fn integration_fail_paused() {
    let owner: AccountId = "owner".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let mut c = Integration::new(owner.clone());

    Integration::pause(&mut c);

    c.set_value(5);
}
