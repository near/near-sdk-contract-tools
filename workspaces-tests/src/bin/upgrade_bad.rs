#![allow(missing_docs)]

workspaces_tests::predicate!();

use near_sdk::{near_bindgen, PanicOnDefault};
use near_sdk_contract_tools::compat_derive_borsh;

compat_derive_borsh! {
    #[derive(PanicOnDefault)]
    #[near_bindgen]
    pub struct ContractBad {
        pub foo: u32,
    }
}

#[near_bindgen]
impl ContractBad {
    #[init]
    pub fn new() -> Self {
        Self { foo: 0 }
    }

    pub fn increment_foo(&mut self) {
        self.foo += 1;
    }

    pub fn get_foo(&self) -> u32 {
        self.foo
    }
}
