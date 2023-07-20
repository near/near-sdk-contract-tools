#![allow(missing_docs)]

// Ignore
pub fn main() {}

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, log, near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::{standard::nep171::*, Nep171};

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, Nep171)]
#[near_bindgen]
pub struct Contract {}

impl Nep171Hook for Contract {
    fn before_nft_transfer(&self, transfer: &Nep171Transfer) {
        log!("before_nft_transfer({})", transfer.token_id);
    }

    fn after_nft_transfer(&mut self, transfer: &Nep171Transfer, _state: ()) {
        log!("after_nft_transfer({})", transfer.token_id);
    }
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }

    pub fn mint(&mut self, token_ids: Vec<TokenId>) {
        Nep171Controller::mint(self, &token_ids, &env::predecessor_account_id(), None)
            .unwrap_or_else(|e| env::panic_str(&format!("Failed to mint: {:#?}", e)));
    }
}
