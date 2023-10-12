#![allow(missing_docs)]

// Ignore
pub fn main() {}

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::{
    standard::{nep141::*, nep148::*},
    FungibleToken,
};

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, FungibleToken)]
#[fungible_token(no_hooks)]
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
        Nep141Controller::mint(self, env::predecessor_account_id(), amount.into(), None).unwrap();
    }
}
