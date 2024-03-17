#![allow(missing_docs)]

workspaces_tests::near_sdk!();
compat_use_borsh!();
use near_sdk::{env, near_bindgen, require, AccountId, PanicOnDefault};
use near_sdk_contract_tools::{compat_derive_borsh, compat_use_borsh};

pub fn main() {} // Ignore

compat_derive_borsh! {
    #[derive(PanicOnDefault)]
    #[near_bindgen]
    pub struct Contract {
        owner_id: AccountId,
        value: String,
        calls: u32,
    }
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            value: "".into(),
            calls: 0,
        }
    }

    pub fn set_value(&mut self, value: String) {
        require!(env::predecessor_account_id() == self.owner_id, "Owner only");
        self.value = value;
        self.calls += 1;
    }

    pub fn get_value(&self) -> &str {
        &self.value
    }

    pub fn get_calls(&self) -> u32 {
        self.calls
    }
}
