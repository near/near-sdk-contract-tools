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
    standard::{nep141::*, nep145::*},
    FungibleToken, Nep145,
};

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, FungibleToken, Nep145)]
#[fungible_token(
    name = "My Fungible Token w/ Storage Management",
    symbol = "MYFT",
    decimals = 24
)]
#[near_bindgen]
pub struct Contract {}

impl Nep145Hook for Contract {
    fn after_force_unregister(
        contract: &mut Self,
        account_id: &near_sdk::AccountId,
        _balance: &StorageBalance,
    ) {
        let balance = Self::balance_of(account_id);
        contract.burn(
            account_id.clone(),
            balance,
            Some("storage force unregister".to_string()),
        );
    }
}

impl Nep141Hook<u64> for Contract {
    fn before_transfer(&mut self, _transfer: &Nep141Transfer) -> u64 {
        env::storage_usage()
    }

    fn after_transfer(&mut self, _transfer: &Nep141Transfer, storage_usage_start: u64) {
        let storage_usage = env::storage_usage() - storage_usage_start;
        let storage_fee = env::storage_byte_cost() * storage_usage as u128;

        Nep145Controller::lock_storage(self, &env::predecessor_account_id(), storage_fee.into())
            .unwrap_or_else(|e| env::panic_str(&format!("Storage lock error: {}", e)));
    }
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }

    pub fn mint(&mut self, amount: U128) {
        self.deposit_unchecked(&env::predecessor_account_id(), amount.into());
    }
}
