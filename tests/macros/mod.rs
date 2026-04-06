use near_sdk::{
    AccountId, BorshStorageKey, PanicOnDefault, env, near, test_utils::VMContextBuilder,
    testing_env,
};
use near_sdk_contract_tools::{
    Escrow, Migrate, Owner, Pause, Rbac, escrow::Escrow, migrate::MigrateHook, owner::Owner,
    pause::Pause, rbac::Rbac, standard::nep297::Event,
};

mod escrow;
mod event;
mod migrate;
mod owner;
mod pause;
mod standard;

mod my_event {
    use near_sdk::{AccountId, serde::Serialize};
    use near_sdk_contract_tools::Nep297;

    #[derive(Serialize, Nep297)]
    #[serde(crate = "near_sdk::serde")]
    #[nep297(standard = "x-myevent", version = "1.0.0", rename = "snake_case")]
    pub struct ValueChanged {
        pub from: u32,
        pub to: u32,
    }

    #[derive(Serialize, Nep297)]
    #[serde(crate = "near_sdk::serde")]
    #[nep297(standard = "x-myevent", version = "1.0.0", rename = "snake_case")]
    pub struct PermissionGranted {
        pub to: AccountId,
    }
}

#[derive(BorshStorageKey)]
#[near]
enum StorageKey {
    Owner,
    Pause,
    Rbac,
}

#[derive(BorshStorageKey)]
#[near]
pub enum Role {
    CanPause,
    CanSetValue,
}

mod integration {
    use super::*;

    #[derive(Owner, Pause, Rbac, Escrow, PanicOnDefault)]
    #[owner(storage_key = "StorageKey::Owner")]
    #[pause(storage_key = "StorageKey::Pause")]
    #[rbac(storage_key = "StorageKey::Rbac", roles = "Role")]
    #[escrow(storage_key = "StorageKey::Owner", id = "u64", state = "String")]
    #[near(contract_state)]
    pub struct Integration {
        pub value: u32,
    }

    #[allow(clippy::needless_pass_by_value)]
    #[near]
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
}
use integration::Integration;

#[derive(Migrate, Owner, Pause, Rbac, PanicOnDefault)]
#[migrate(from = "Integration")]
#[owner(storage_key = "StorageKey::Owner")]
#[pause(storage_key = "StorageKey::Pause")]
#[rbac(storage_key = "StorageKey::Rbac", roles = "Role")]
#[near(contract_state)]
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

#[near]
impl MigrateIntegration {
    pub fn add_value_setter(&mut self, account_id: AccountId) {
        Self::require_owner();

        self.add_role(&account_id, &Role::CanSetValue);

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

    let mut migrated = MigrateIntegration::migrate();

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

    MigrateIntegration::migrate();
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

    MigrateIntegration::migrate();
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
        AccountId, NearToken, PanicOnDefault, env, near, test_utils::VMContextBuilder, testing_env,
    };
    use near_sdk_contract_tools::{
        Pause,
        ft::*,
        hook::Hook,
        pause::{Pause, hooks::Pausable},
    };

    #[derive(FungibleToken, Pause, PanicOnDefault)]
    #[fungible_token(all_hooks = "Pausable", transfer_hook = "TransferHook")]
    #[near(contract_state)]
    struct Contract {
        pub storage_usage: u64,
    }

    #[near]
    impl Contract {
        #[init]
        pub fn new() -> Self {
            let mut contract = Self { storage_usage: 0 };

            contract.set_metadata(&ContractMetadata::new("Pausable Fungible Token", "PFT", 18));

            contract
        }
    }

    #[derive(Default)]
    struct TransferHook;

    impl Hook<Contract, Nep141Transfer<'_>> for TransferHook {
        fn hook<R>(
            contract: &mut Contract,
            _args: &Nep141Transfer,
            f: impl FnOnce(&mut Contract) -> R,
        ) -> R {
            let state = env::storage_usage();
            let r = f(contract);
            let storage_delta = env::storage_usage() - state;
            println!("Storage delta: {storage_delta}");

            contract.storage_usage = storage_delta;
            r
        }
    }

    #[test]
    fn hooks_modify_state() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob_account".parse().unwrap();

        let mut c = Contract::new();

        let context = VMContextBuilder::new()
            .attached_deposit(NearToken::from_near(1).saturating_div(100))
            .predecessor_account_id(alice.clone())
            .build();
        testing_env!(context);
        c.storage_deposit(None, None);
        let context = VMContextBuilder::new()
            .attached_deposit(NearToken::from_near(1).saturating_div(100))
            .predecessor_account_id(bob.clone())
            .build();
        testing_env!(context);
        c.storage_deposit(None, None);

        c.deposit_unchecked(&alice, 100).unwrap();

        let context = VMContextBuilder::new()
            .attached_deposit(NearToken::from_yoctonear(1))
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

        let mut c = Contract::new();

        c.deposit_unchecked(&alice, 100).unwrap();

        let context = VMContextBuilder::new()
            .attached_deposit(NearToken::from_yoctonear(1))
            .predecessor_account_id(alice.clone())
            .build();

        testing_env!(context);

        Pause::pause(&mut c);

        c.ft_transfer(bob.clone(), 50.into(), None);
    }
}

#[cfg(test)]
mod owned_fungible_token {
    use near_sdk::{
        AccountId, env, json_types::U128, near, test_utils::VMContextBuilder, testing_env,
    };
    use near_sdk::{NearToken, PanicOnDefault};
    use near_sdk_contract_tools::{
        Owner,
        ft::*,
        owner::{hooks::OnlyOwner, *},
    };

    #[derive(Owner, FungibleToken, PanicOnDefault)]
    #[fungible_token(all_hooks = "OnlyOwner")] // only the owner can transfer, etc. the tokens
    #[near(contract_state)]
    pub struct Contract {}

    #[near]
    impl Contract {
        #[init]
        pub fn new() -> Self {
            let mut contract = Self {};

            Owner::init(&mut contract, &env::predecessor_account_id());

            contract
        }

        pub fn mint(&mut self, amount: U128) {
            Nep141Controller::mint(
                self,
                &Nep141Mint::new(amount.0, env::predecessor_account_id()),
            )
            .unwrap();
        }
    }

    #[test]
    fn mint_and_transfer() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob".parse().unwrap();
        testing_env!(
            VMContextBuilder::new()
                .predecessor_account_id(alice.clone())
                .build()
        );

        let mut contract = Contract::new(); // since alice is the predecessor during init, alice is the owner

        // internal method calls
        contract
            .deposit_to_storage_account(&alice, NearToken::from_near(1))
            .unwrap();
        contract
            .deposit_to_storage_account(&bob, NearToken::from_near(1))
            .unwrap();

        // external; alice is still predecessor
        contract.mint(U128(100));

        assert_eq!(contract.ft_balance_of(alice.clone()), U128(100));
        assert_eq!(contract.ft_balance_of(bob.clone()), U128(0));

        testing_env!(
            VMContextBuilder::new()
                .predecessor_account_id(alice.clone())
                .attached_deposit(NearToken::from_yoctonear(1u128))
                .build()
        );
        contract.ft_transfer(bob.clone(), U128(10), None);

        assert_eq!(contract.ft_balance_of(alice), U128(90));
        assert_eq!(contract.ft_balance_of(bob), U128(10));
    }

    #[test]
    #[should_panic = "Owner only"]
    fn mint_fail_not_owner() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob".parse().unwrap();
        testing_env!(
            VMContextBuilder::new()
                .predecessor_account_id(alice.clone())
                .build()
        );

        let mut contract = Contract::new(); // since alice is the predecessor during init, alice is the owner

        // internal method calls
        contract
            .deposit_to_storage_account(&alice, NearToken::from_near(1))
            .unwrap();
        contract
            .deposit_to_storage_account(&bob, NearToken::from_near(1))
            .unwrap();

        testing_env!(VMContextBuilder::new().predecessor_account_id(bob).build());

        contract.mint(U128(100));
    }

    #[test]
    #[should_panic = "Owner only"]
    fn transfer_fail_not_owner() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob".parse().unwrap();
        testing_env!(
            VMContextBuilder::new()
                .predecessor_account_id(alice.clone())
                .build()
        );

        let mut contract = Contract::new(); // since alice is the predecessor during init, alice is the owner

        // internal method calls
        contract
            .deposit_to_storage_account(&alice, NearToken::from_near(1))
            .unwrap();
        contract
            .deposit_to_storage_account(&bob, NearToken::from_near(1))
            .unwrap();

        Nep141Controller::deposit_unchecked(&mut contract, &bob, 100).unwrap();

        testing_env!(
            VMContextBuilder::new()
                .predecessor_account_id(bob)
                .attached_deposit(NearToken::from_yoctonear(1))
                .build()
        );
        contract.ft_transfer(alice, U128(10), None);
    }
}
