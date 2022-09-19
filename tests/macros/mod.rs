use near_contract_tools::{
    migrate::{MigrateExternal, MigrateHook},
    owner::Owner,
    pause::Pause,
    rbac::Rbac,
    standard::nep297::Event,
    Migrate, Nep297, Owner, Pause, Rbac,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen,
    test_utils::VMContextBuilder,
    testing_env, AccountId, BorshStorageKey,
};
use serde::Serialize;

mod event;
mod migrate;
mod owner;
mod pause;
mod standard;

#[derive(Serialize, Nep297)]
#[nep297(standard = "x-myevent", version = "1.0.0", rename_all = "snake_case")]
#[serde(untagged)]
enum MyEvent {
    ValueChanged { from: u32, to: u32 },
    PermissionGranted { to: AccountId },
}

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

#[derive(Owner, Pause, Rbac, BorshDeserialize, BorshSerialize)]
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

        Owner::init(&mut contract, &owner_id);
        contract.add_role(&owner_id, &Role::CanSetValue);
        contract.add_role(&owner_id, &Role::CanPause);

        contract
    }

    pub fn add_value_setter(&mut self, account_id: AccountId) {
        Self::require_owner();

        self.add_role(&account_id, &Role::CanSetValue);

        MyEvent::PermissionGranted { to: account_id }.emit();
    }

    pub fn set_value(&mut self, value: u32) {
        Self::require_unpaused();
        self.require_role(&Role::CanSetValue);

        let old = self.value;

        self.value = value;

        MyEvent::ValueChanged {
            from: old,
            to: value,
        }
        .emit();
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

#[derive(Migrate, Owner, Pause, Rbac, BorshSerialize, BorshDeserialize)]
#[migrate(from = "Integration")]
#[owner(storage_key = "StorageKey::Owner")]
#[pause(storage_key = "StorageKey::Pause")]
#[rbac(storage_key = "StorageKey::Rbac", roles = "Role")]
#[near_bindgen]
struct MigrateIntegration {
    pub new_value: String,
    pub moved_value: u32,
}

impl MigrateHook for MigrateIntegration {
    fn migrate(old: Integration, _args: Option<String>) -> Self {
        Self::require_owner();
        Self::require_unpaused();

        Self {
            new_value: "my string".to_string(),
            moved_value: old.value,
        }
    }
}

#[near_bindgen]
impl MigrateIntegration {
    pub fn add_value_setter(&mut self, account_id: AccountId) {
        Self::require_owner();

        self.add_role(&account_id, &Role::CanSetValue);

        MyEvent::PermissionGranted { to: account_id }.emit();
    }

    pub fn set_value(&mut self, value: u32) {
        Self::require_unpaused();
        self.require_role(&Role::CanSetValue);

        let old = self.moved_value;

        self.moved_value = value;

        MyEvent::ValueChanged {
            from: old,
            to: value,
        }
        .emit();
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
        self.moved_value
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

    // Perform migration
    env::state_write(&c);

    let mut migrated = <MigrateIntegration as MigrateExternal>::migrate(None);

    assert_eq!(migrated.moved_value, 25);
    assert_eq!(migrated.get_value(), 25);
    assert_eq!(migrated.new_value, "my string");

    let bob: AccountId = "bob_addr".parse().unwrap();

    migrated.set_value(5);

    assert_eq!(migrated.get_value(), 5);

    // make sure alice still has permission
    let context = VMContextBuilder::new()
        .predecessor_account_id(alice.clone())
        .build();

    testing_env!(context);

    migrated.set_value(256);

    assert_eq!(migrated.get_value(), 256);

    // add bob permissions
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);

    migrated.add_value_setter(bob.clone());

    let context = VMContextBuilder::new()
        .predecessor_account_id(bob.clone())
        .build();

    testing_env!(context);

    migrated.set_value(77);

    assert_eq!(migrated.get_value(), 77);

    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);

    MigrateIntegration::pause(&mut migrated);
    MigrateIntegration::unpause(&mut migrated);

    migrated.set_value(8);

    assert_eq!(migrated.get_value(), 8);
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
fn integration_fail_set_paused() {
    let owner: AccountId = "owner".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let mut c = Integration::new(owner.clone());

    Integration::pause(&mut c);

    c.set_value(5);
}

#[test]
#[should_panic(expected = "Owner only")]
fn integration_fail_migrate_allow() {
    let owner: AccountId = "owner".parse().unwrap();
    let alice: AccountId = "alice".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let c = Integration::new(owner.clone());

    env::state_write(&c);

    let context = VMContextBuilder::new()
        .predecessor_account_id(alice.clone())
        .build();

    testing_env!(context);

    <MigrateIntegration as MigrateExternal>::migrate(None);
}

#[test]
#[should_panic(expected = "Disallowed while contract is paused")]
fn integration_fail_migrate_paused() {
    let owner: AccountId = "owner".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let mut c = Integration::new(owner.clone());

    Integration::pause(&mut c);

    env::state_write(&c);

    <MigrateIntegration as MigrateExternal>::migrate(None);
}

#[cfg(test)]
mod pausable_fungible_token {
    use near_contract_tools::{
        pause::Pause,
        standard::nep141::{Nep141Hook, Nep141Transfer},
        FungibleToken, Pause,
    };
    use near_sdk::{
        borsh::{self, BorshDeserialize, BorshSerialize},
        env, near_bindgen,
        test_utils::VMContextBuilder,
        testing_env, AccountId,
    };

    #[derive(FungibleToken, Pause, BorshDeserialize, BorshSerialize)]
    #[fungible_token(name = "Pausable Fungible Token", symbol = "PFT", decimals = 18)]
    #[near_bindgen]
    struct Contract {
        pub storage_usage: u64,
    }

    #[derive(Default)]
    struct HookState {
        pub storage_usage_start: u64,
    }

    impl Nep141Hook<HookState> for Contract {
        fn before_transfer(&mut self, _transfer: &Nep141Transfer) -> HookState {
            Contract::require_unpaused();
            HookState {
                storage_usage_start: env::storage_usage(),
            }
        }

        fn after_transfer(&mut self, _transfer: &Nep141Transfer, state: HookState) {
            let storage_delta = env::storage_usage() - state.storage_usage_start;
            println!("Storage delta: {storage_delta}",);

            self.storage_usage = storage_delta;
        }
    }

    #[test]
    fn hooks_modify_state() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob_account".parse().unwrap();

        let mut c = Contract { storage_usage: 0 };

        c.deposit_unchecked(&alice, 100);

        let context = VMContextBuilder::new()
            .attached_deposit(1)
            .predecessor_account_id(alice.clone())
            .build();

        testing_env!(context);

        c.ft_transfer(bob.clone(), 50.into(), None);

        assert_ne!(c.storage_usage, 0);
    }

    #[test]
    #[should_panic(expected = "Disallowed while contract is paused")]
    fn hooks_can_terminate_on_error() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob_account".parse().unwrap();

        let mut c = Contract { storage_usage: 0 };

        c.deposit_unchecked(&alice, 100);

        let context = VMContextBuilder::new()
            .attached_deposit(1)
            .predecessor_account_id(alice.clone())
            .build();

        testing_env!(context);

        Pause::pause(&mut c);

        c.ft_transfer(bob.clone(), 50.into(), None);
    }
}
