compat_use_borsh!();
use near_sdk::{
    env, near_bindgen, test_utils::VMContextBuilder, testing_env, AccountId, BorshStorageKey,
};
use near_sdk_contract_tools::{
    compat_derive_borsh, compat_derive_storage_key, compat_use_borsh, escrow::Escrow,
    migrate::MigrateHook, owner::Owner, pause::Pause, rbac::Rbac, standard::nep297::Event, Escrow,
    Migrate, Owner, Pause, Rbac,
};

mod escrow;
mod event;
mod migrate;
mod owner;
mod pause;
mod standard;

mod my_event {
    use near_sdk::{serde::Serialize, AccountId};
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

compat_derive_storage_key! {
    enum StorageKey {
        Owner,
        Pause,
        Rbac,
    }
}

compat_derive_storage_key! {
    pub enum Role {
        CanPause,
        CanSetValue,
    }
}

mod integration {
    use super::*;

    compat_derive_borsh! {
        #[derive(Owner, Pause, Rbac, Escrow)]
        #[owner(storage_key = "StorageKey::Owner")]
        #[pause(storage_key = "StorageKey::Pause")]
        #[rbac(storage_key = "StorageKey::Rbac", roles = "Role")]
        #[escrow(storage_key = "StorageKey::Owner", id = "u64", state = "String")]
        #[near_bindgen]
        pub struct Integration {
            pub value: u32,
        }
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

compat_derive_borsh! {
    #[derive(Migrate, Owner, Pause, Rbac)]
    #[migrate(from = "Integration")]
    #[owner(storage_key = "StorageKey::Owner")]
    #[pause(storage_key = "StorageKey::Pause")]
    #[rbac(storage_key = "StorageKey::Rbac", roles = "Role")]
    #[near_bindgen]
    struct MigrateIntegration {
        pub new_value: String,
        pub moved_value: u32,
    }
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
    compat_use_borsh!();
    use near_sdk::{env, near_bindgen, test_utils::VMContextBuilder, testing_env, AccountId};
    use near_sdk_contract_tools::{
        compat_derive_borsh, compat_use_borsh,
        ft::*,
        hook::Hook,
        pause::{hooks::PausableHook, Pause},
        Pause, COMPAT_ONE_NEAR, COMPAT_ONE_YOCTONEAR,
    };

    compat_derive_borsh! {
        #[derive(FungibleToken, Pause)]
        #[fungible_token(all_hooks = "PausableHook", transfer_hook = "TransferHook")]
        #[near_bindgen]
        struct Contract {
            pub storage_usage: u64,
        }
    }

    #[near_bindgen]
    impl Contract {
        #[init]
        pub fn new() -> Self {
            let mut contract = Self { storage_usage: 0 };

            contract.set_metadata(&FungibleTokenMetadata::new(
                "Pausable Fungible Token".into(),
                "PFT".into(),
                18,
            ));

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
            .attached_deposit(COMPAT_ONE_NEAR.saturating_div(100))
            .predecessor_account_id(alice.clone())
            .build();
        testing_env!(context);
        c.storage_deposit(None, None);
        let context = VMContextBuilder::new()
            .attached_deposit(COMPAT_ONE_NEAR.saturating_div(100))
            .predecessor_account_id(bob.clone())
            .build();
        testing_env!(context);
        c.storage_deposit(None, None);

        c.deposit_unchecked(&alice, 100).unwrap();

        let context = VMContextBuilder::new()
            .attached_deposit(*COMPAT_ONE_YOCTONEAR)
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
            .attached_deposit(*COMPAT_ONE_YOCTONEAR)
            .predecessor_account_id(alice.clone())
            .build();

        testing_env!(context);

        Pause::pause(&mut c);

        c.ft_transfer(bob.clone(), 50.into(), None);
    }
}

#[cfg(test)]
mod owned_fungible_token {
    compat_use_borsh!();
    use near_sdk::{
        env, json_types::U128, near_bindgen, test_utils::VMContextBuilder, testing_env, AccountId,
        PanicOnDefault,
    };
    use near_sdk_contract_tools::{
        compat_derive_borsh, compat_near_to_u128, compat_use_borsh,
        ft::*,
        owner::{hooks::OnlyOwner, *},
        Owner, COMPAT_ONE_NEAR, COMPAT_ONE_YOCTONEAR,
    };

    compat_derive_borsh! {
        #[derive(PanicOnDefault, Owner, FungibleToken)]
        #[fungible_token(all_hooks = "OnlyOwner")] // only the owner can transfer, etc. the tokens
        #[near_bindgen]
        pub struct Contract {}
    }

    #[near_bindgen]
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
                &Nep141Mint {
                    amount: amount.into(),
                    receiver_id: &env::predecessor_account_id(),
                    memo: None,
                },
            )
            .unwrap();
        }
    }

    #[test]
    fn mint_and_transfer() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob".parse().unwrap();
        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(alice.clone())
            .build());

        let mut contract = Contract::new(); // since alice is the predecessor during init, alice is the owner

        // internal method calls
        contract
            .deposit_to_storage_account(&alice, compat_near_to_u128!(*COMPAT_ONE_NEAR).into())
            .unwrap();
        contract
            .deposit_to_storage_account(&bob, compat_near_to_u128!(*COMPAT_ONE_NEAR).into())
            .unwrap();

        // external; alice is still predecessor
        contract.mint(U128(100));

        assert_eq!(contract.ft_balance_of(alice.clone()), U128(100));
        assert_eq!(contract.ft_balance_of(bob.clone()), U128(0));

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(alice.clone())
            .attached_deposit(*COMPAT_ONE_YOCTONEAR)
            .build());
        contract.ft_transfer(bob.clone(), U128(10), None);

        assert_eq!(contract.ft_balance_of(alice), U128(90));
        assert_eq!(contract.ft_balance_of(bob), U128(10));
    }

    #[test]
    #[should_panic = "Owner only"]
    fn mint_fail_not_owner() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob".parse().unwrap();
        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(alice.clone())
            .build());

        let mut contract = Contract::new(); // since alice is the predecessor during init, alice is the owner

        // internal method calls
        contract
            .deposit_to_storage_account(&alice, compat_near_to_u128!(*COMPAT_ONE_NEAR).into())
            .unwrap();
        contract
            .deposit_to_storage_account(&bob, compat_near_to_u128!(*COMPAT_ONE_NEAR).into())
            .unwrap();

        testing_env!(VMContextBuilder::new().predecessor_account_id(bob).build());

        contract.mint(U128(100));
    }

    #[test]
    #[should_panic = "Owner only"]
    fn transfer_fail_not_owner() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob".parse().unwrap();
        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(alice.clone())
            .build());

        let mut contract = Contract::new(); // since alice is the predecessor during init, alice is the owner

        // internal method calls
        contract
            .deposit_to_storage_account(&alice, compat_near_to_u128!(*COMPAT_ONE_NEAR).into())
            .unwrap();
        contract
            .deposit_to_storage_account(&bob, compat_near_to_u128!(*COMPAT_ONE_NEAR).into())
            .unwrap();

        Nep141Controller::deposit_unchecked(&mut contract, &bob, 100).unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(bob)
            .attached_deposit(*COMPAT_ONE_YOCTONEAR)
            .build());
        contract.ft_transfer(alice, U128(10), None);
    }
}
