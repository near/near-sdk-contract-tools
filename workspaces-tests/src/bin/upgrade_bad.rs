#![allow(missing_docs)]

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    near_bindgen, PanicOnDefault,
};

pub fn main() {} // Ignore

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
#[near_bindgen]
pub struct ContractBad {
    pub foo: u32,
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
