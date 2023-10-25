#![allow(missing_docs)]

// Ignore
pub fn main() {}

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::ft::*;

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, FungibleToken)]
#[near_bindgen]
pub struct Contract {}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self {};

        contract.set_metadata(&FungibleTokenMetadata::new(
            "My Fungible Token".into(),
            "MYFT".into(),
            24,
        ));

        contract
    }

    pub fn mint(&mut self, amount: U128) {
        Nep141Controller::mint(
            self,
            &Nep141Mint {
                amount: amount.into(),
                account_id: &env::predecessor_account_id(),
                memo: None,
            },
        )
        .unwrap();
    }
}
