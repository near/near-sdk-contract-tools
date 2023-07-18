#![allow(missing_docs)]

// Ignore
pub fn main() {}

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, log, near_bindgen, AccountId, PanicOnDefault, PromiseOrValue,
};
use near_sdk_contract_tools::{standard::nep171::*, Nep171};

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize)]
#[near_bindgen]
pub struct Contract {}

#[near_bindgen]
impl Nep171Receiver for Contract {
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: TokenId,
        msg: String,
    ) -> PromiseOrValue<bool> {
        log!(
            "Received {} from {} via {}",
            token_id,
            previous_owner_id,
            sender_id,
        );

        PromiseOrValue::Value(msg == "return")
    }
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }
}
