#![allow(missing_docs)]

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    near_bindgen, PanicOnDefault,
};

pub fn main() {} // Ignore

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize)]
#[near_bindgen]
pub struct Contract {}

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
