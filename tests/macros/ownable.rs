use near_contract_tools::{owner::Owner, Owner};
use near_sdk::{
    borsh::{self, BorshSerialize},
    env, near_bindgen,
    test_utils::VMContextBuilder,
    testing_env, AccountId, BorshStorageKey,
};

#[derive(Owner)]
#[near_bindgen]
pub struct OwnedStructImplicitKey {
    pub permissioned_item: u32,
}

#[near_bindgen]
impl OwnedStructImplicitKey {
    #[init]
    pub fn new() -> Self {
        let contract = Self {
            permissioned_item: 0,
        };

        // This method can only be called once throughout the entire duration of the contract
        Owner::init(&contract, env::predecessor_account_id());

        contract
    }

    pub fn set_permissioned_item(&mut self, value: u32) {
        self.require_owner();

        self.permissioned_item = value;
    }

    pub fn get_permissioned_item(&self) -> u32 {
        self.permissioned_item
    }
}

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    MyStorageKey,
}

#[derive(Owner)]
#[owner(storage_key = "StorageKey::MyStorageKey")]
#[near_bindgen]
pub struct OwnedStructExplicitKey {
    pub permissioned_item: u32,
}

#[near_bindgen]
impl OwnedStructExplicitKey {
    #[init]
    pub fn new() -> Self {
        let contract = Self {
            permissioned_item: 0,
        };

        // This method can only be called once throughout the entire duration of the contract
        Owner::init(&contract, env::predecessor_account_id());

        contract
    }

    pub fn try_init_again(&self) {
        // Should fail
        Owner::init(self, env::predecessor_account_id());
    }

    pub fn set_permissioned_item(&mut self, value: u32) {
        self.require_owner();

        self.permissioned_item = value;
    }

    pub fn get_permissioned_item(&self) -> u32 {
        self.permissioned_item
    }
}

#[test]
fn derive_ownable_im() {
    let owner: AccountId = "owner".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let mut c = OwnedStructImplicitKey::new();

    assert_eq!(
        c.own_get_owner(),
        Some(owner.clone()),
        "Owner is initialized",
    );

    c.set_permissioned_item(4);

    assert_eq!(
        c.get_permissioned_item(),
        4,
        "Permissioned item set correctly",
    );
}

#[test]
#[should_panic(expected = "Owner only")]
fn derive_ownable_im_unauthorized() {
    let owner: AccountId = "owner".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let mut c = OwnedStructImplicitKey::new();

    let alice: AccountId = "alice".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(alice.clone())
        .build();
    testing_env!(context);

    // Alice is not authorized to call owner-only method
    c.set_permissioned_item(4);
}

#[test]
fn derive_ownable_ex() {
    let owner: AccountId = "owner".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let mut c = OwnedStructExplicitKey::new();

    assert_eq!(
        c.own_get_owner(),
        Some(owner.clone()),
        "Owner is initialized",
    );

    c.set_permissioned_item(4);

    assert_eq!(
        c.get_permissioned_item(),
        4,
        "Permissioned item set correctly",
    );
}

#[test]
#[should_panic(expected = "Owner already initialized")]
fn derive_ownable_ex_init_again() {
    let owner: AccountId = "owner".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let c = OwnedStructExplicitKey::new();

    c.try_init_again();
}

#[test]
#[should_panic(expected = "Owner only")]
fn derive_ownable_ex_unauthorized() {
    let owner: AccountId = "owner".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let mut c = OwnedStructExplicitKey::new();

    let alice: AccountId = "alice".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(alice.clone())
        .build();
    testing_env!(context);

    // Alice is not authorized to call owner-only method
    c.set_permissioned_item(4);
}
