#![allow(missing_docs)]

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    near_bindgen, PanicOnDefault,
};

pub fn main() {}

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
#[near_bindgen]
pub struct Contract {}

#[near_bindgen]
impl Contract {
    pub fn say_hello(&self) -> String {
        "I am the second contract".to_string()
    }

    #[init(ignore_state)]
    pub fn migrate() -> Self {
        near_sdk::env::log_str("migrate called!");
        Self {}
    }
}
