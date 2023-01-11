#![allow(missing_docs)]

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    near_bindgen,
    store::Vector,
    PanicOnDefault, Promise, PromiseOrValue,
};
use near_sdk_contract_tools::utils::apply_storage_fee_and_refund;

pub fn main() {} // Ignore

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
#[near_bindgen]
pub struct ContractBad {
    pub items: Vector<String>,
}

#[near_bindgen]
impl ContractBad {
    #[init]
    pub fn new() -> Self {
        Self {
            items: Vector::new(b"i"),
        }
    }

    pub fn storage_byte_cost(&self) -> U128 {
        env::storage_byte_cost().into()
    }

    #[payable]
    pub fn store(&mut self, item: String) -> Option<Promise> {
        let initial_storage_usage = env::storage_usage();

        self.items.push(item);

        self.items.flush(); // Force write before sending refund

        apply_storage_fee_and_refund(initial_storage_usage, 0)
    }
}
