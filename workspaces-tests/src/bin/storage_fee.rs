#![allow(missing_docs)]

workspaces_tests::predicate!();
use near_sdk::{env, json_types::U128, near_bindgen, store::Vector, PanicOnDefault, Promise};
use near_sdk_contract_tools::{
    compat_derive_borsh, compat_near_to_u128, utils::apply_storage_fee_and_refund,
};

compat_derive_borsh! {
    #[derive(PanicOnDefault)]
    #[near_bindgen]
    pub struct ContractBad {
        pub items: Vector<String>,
    }
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
        compat_near_to_u128!(env::storage_byte_cost()).into()
    }

    #[payable]
    pub fn store(&mut self, item: String) -> Option<Promise> {
        let initial_storage_usage = env::storage_usage();

        self.items.push(item);

        self.items.flush(); // Force write before sending refund

        apply_storage_fee_and_refund(initial_storage_usage, 0)
    }
}
