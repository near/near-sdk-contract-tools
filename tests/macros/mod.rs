use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen,
    test_utils::VMContextBuilder,
    testing_env, AccountId, BorshStorageKey,
};
use near_sdk_contract_tools::escrow::Escrow;
use near_sdk_contract_tools::{
    migrate::{MigrateExternal, MigrateHook},
    owner::Owner,
    pause::Pause,
    rbac::Rbac,
    standard::nep297::Event,
    Escrow, Migrate, Owner, Pause, Rbac,
};

mod escrow;
mod event;
mod migrate;
mod owner;
mod pause;
mod standard;

mod my_event {
    use near_sdk::AccountId;
    use near_sdk_contract_tools::Nep297;
    use serde::Serialize;

    #[derive(Serialize, Nep297)]
    #[nep297(standard = "x-myevent", version = "1.0.0", rename = "snake_case")]
    pub struct ValueChanged {
        pub from: u32,
        pub to: u32,
    }

    #[derive(Serialize, Nep297)]
    #[nep297(standard = "x-myevent", version = "1.0.0", rename = "snake_case")]
    pub struct PermissionGranted {
        pub to: AccountId,
    }
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

#[derive(Owner, Pause, Rbac, Escrow, BorshDeserialize, BorshSerialize)]
#[owner(storage_key = "StorageKey::Owner")]
#[pause(storage_key = "StorageKey::Pause")]
#[rbac(storage_key = "StorageKey::Rbac", roles = "Role")]
#[escrow(storage_key = "StorageKey::Owner", id = "u64", state = "String")]
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
        contract.add_role(owner_id.clone(), &Role::CanSetValue);
        contract.add_role(owner_id.clone(), &Role::CanPause);

        contract
    }

    pub fn add_value_setter(&mut self, account_id: AccountId) {
        Self::require_owner();

        self.add_role(account_id.clone(), &Role::CanSetValue);

        my_event::PermissionGranted { to: account_id }.emit();
    }

    pub fn set_value(&mut self, value: u32) {
        Self::require_unpaused();
        Self::require_role(&Role::CanSetValue);

        let old = self.value;

        self.value = value;

        my_event::ValueChanged {
            from: old,
            to: value,
        }
        .emit();
    }

    pub fn pause(&mut self) {
        Self::require_role(&Role::CanPause);
        Pause::pause(self);
    }

    pub fn unpause(&mut self) {
        Self::require_role(&Role::CanPause);
        Pause::unpause(self);
    }

    pub fn get_value(&self) -> u32 {
        self.value
    }

    pub fn lock_data(&mut self, id: u64, data: String) {
        self.lock(&id, &data);
    }

    pub fn unlock_data(&mut self, id: u64) {
        self.unlock(&id, |data| !data.is_empty());
    }

    pub fn check_is_locked(&self, id: u64) -> bool {
        self.is_locked(&id)
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
    fn on_migrate(old: Integration) -> Self {
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

        self.add_role(account_id.clone(), &Role::CanSetValue);

        my_event::PermissionGranted { to: account_id }.emit();
    }

    pub fn set_value(&mut self, value: u32) {
        Self::require_unpaused();
        Self::require_role(&Role::CanSetValue);

        let old = self.moved_value;

        self.moved_value = value;

        my_event::ValueChanged {
            from: old,
            to: value,
        }
        .emit();
    }

    pub fn pause(&mut self) {
        Self::require_role(&Role::CanPause);
        Pause::pause(self);
    }

    pub fn unpause(&mut self) {
        Self::require_role(&Role::CanPause);
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

    let mut migrated = <MigrateIntegration as MigrateExternal>::migrate();

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

    c.lock_data(1, "Data".to_string());
    assert!(c.check_is_locked(1));
    c.unlock_data(1);
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

    <MigrateIntegration as MigrateExternal>::migrate();
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

    <MigrateIntegration as MigrateExternal>::migrate();
}
#[test]
#[should_panic(expected = "Already locked")]
fn integration_fail_cannot_lock_twice() {
    let owner: AccountId = "owner".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let mut c = Integration::new(owner.clone());

    let id = 1;
    let data = "Data".to_string();
    c.lock_data(id, data.clone());
    c.lock_data(id, data.clone());
}

#[cfg(test)]
mod pausable_fungible_token {
    use near_sdk::{
        borsh::{self, BorshDeserialize, BorshSerialize},
        env, near_bindgen,
        test_utils::VMContextBuilder,
        testing_env, AccountId,
    };
    use near_sdk_contract_tools::{
        pause::Pause,
        standard::nep141::{Nep141, Nep141Controller, Nep141Hook, Nep141Transfer},
        FungibleToken, Pause,
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

    impl Nep141Hook for Contract {
        type MintState = ();
        type TransferState = HookState;
        type BurnState = ();

        fn before_mint(_contract: &Self, _amount: u128, _account_id: &AccountId) {}

        fn after_mint(_contract: &mut Self, _amount: u128, _account_id: &AccountId, _: ()) {}

        fn before_burn(_contract: &Self, _amount: u128, _account_id: &AccountId) {}

        fn after_burn(_contract: &mut Self, _amount: u128, _account_id: &AccountId, _: ()) {}

        fn before_transfer(_contract: &Self, _transfer: &Nep141Transfer) -> HookState {
            Contract::require_unpaused();
            HookState {
                storage_usage_start: env::storage_usage(),
            }
        }

        fn after_transfer(contract: &mut Self, _transfer: &Nep141Transfer, state: HookState) {
            let storage_delta = env::storage_usage() - state.storage_usage_start;
            println!("Storage delta: {storage_delta}");

            contract.storage_usage = storage_delta;
        }
    }

    #[test]
    fn hooks_modify_state() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob_account".parse().unwrap();

        let mut c = Contract { storage_usage: 0 };

        c.deposit_unchecked(&alice, 100).unwrap();

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

        c.deposit_unchecked(&alice, 100).unwrap();

        let context = VMContextBuilder::new()
            .attached_deposit(1)
            .predecessor_account_id(alice.clone())
            .build();

        testing_env!(context);

        Pause::pause(&mut c);

        c.ft_transfer(bob.clone(), 50.into(), None);
    }
}
