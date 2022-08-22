// Ignore
pub fn main() {}

use near_contract_tools::FungibleToken;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    near_bindgen, PanicOnDefault,
};

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, FungibleToken)]
#[fungible_token(name = "My Fungible Token", symbol = "MYFT", decimals = 18, no_hooks)]
#[near_bindgen]
pub struct Contract {}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }

    pub fn mint(&mut self, amount: U128) {
        self.unchecked_deposit(&env::predecessor_account_id(), amount.into());
    }
}
