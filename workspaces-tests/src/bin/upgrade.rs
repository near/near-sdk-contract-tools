#![allow(missing_docs)]

// Ignore
pub fn main() {}

use std::fmt::Display;

use near_contract_tools::{
    rbac::Rbac,
    slot::Slot,
    upgrade::{
        upgrade::{self, Upgrade},
        Upgrade, UpgradeHook,
    },
    Rbac,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault,
};
use thiserror::Error;

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Upgrade,
}

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, Upgrade)]
#[near_bindgen]
pub struct Contract {
    uint32: var1,
}

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, Upgrade)]
#[near_bindgen]
pub struct ContractV2 {
    uint32: var2,
}


// This single function implementation completely implements simple multisig on
// the contract
impl Upgrade for Contract {
    fn upgrade() {
        env::log_str("Upgrading contract");
        Contract::upgrade();
    }
}

impl UpgradeHook for Contract {
    fn on_upgrade(&mut self) {
        env::log_str("Upgraded!");
    }
}

#[derive(Clone, Copy, Debug)]
pub enum UpgradeError {
    UnauthorizedAccount,
}

impl Display for UpgradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unauthorized account")
    }
}

#[derive(Error, Clone, Debug)]
#[error("Missing role: {0:?}")]
pub struct MissingRole(Role);

impl AccountAuthorizer for Contract {
    type AuthorizationError = MissingRole;

    fn is_account_authorized(account_id: &AccountId) -> Result<(), Self::AuthorizationError> {
        if Contract::has_role(account_id, &Role::Multisig) {
            Ok(())
        } else {
            Err(MissingRole(Role::Multisig))
        }
    }
}

impl Migrate for Contract {
    fn migrate(&mut self) {
        env::log_str("Migrating contract");
        self.uint32 = 2;
    }
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        let contract = Self {};
        contract
    }
}
