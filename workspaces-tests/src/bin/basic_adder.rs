#![allow(missing_docs)]

workspaces_tests::predicate!();

use near_sdk::{near_bindgen, PanicOnDefault};
use near_sdk_contract_tools::compat_derive_borsh;

compat_derive_borsh! {
    #[derive(PanicOnDefault)]
    #[near_bindgen]
    pub struct Contract {}
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }

    pub fn add_five(&self, value: u32) -> u32 {
        value + 5
    }
}
