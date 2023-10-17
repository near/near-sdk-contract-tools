#![allow(missing_docs)]

// Ignore
pub fn main() {}

use std::cmp::Ordering;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    near_bindgen, require, AccountId, PanicOnDefault,
};
use near_sdk_contract_tools::{ft::*, standard::nep145::*, Nep145};

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, FungibleToken, Nep145)]
#[near_bindgen]
pub struct Contract {}

impl Nep145Hook for Contract {
    fn after_force_unregister(
        contract: &mut Self,
        account_id: &AccountId,
        _balance: &StorageBalance,
    ) {
        let balance = contract.balance_of(account_id);
        contract
            .burn(
                account_id.clone(),
                balance,
                Some("storage force unregister".to_string()),
            )
            .unwrap();
    }
}

impl Nep141Hook for Contract {
    type MintState = u64;
    type TransferState = u64;
    type BurnState = u64;

    fn before_mint(contract: &Self, _amount: u128, account_id: &AccountId) -> u64 {
        contract.require_registration(account_id);
        env::storage_usage()
    }

    fn after_mint(
        contract: &mut Self,
        _amount: u128,
        _account_id: &AccountId,
        storage_usage_start: u64,
    ) {
        contract.storage_accounting(storage_usage_start);
    }

    fn before_transfer(contract: &Self, transfer: &Nep141Transfer) -> u64 {
        contract.require_registration(&transfer.receiver_id);
        env::storage_usage()
    }

    fn after_transfer(contract: &mut Self, _transfer: &Nep141Transfer, storage_usage_start: u64) {
        contract.storage_accounting(storage_usage_start);
    }

    fn before_burn(_contract: &Self, _amount: u128, _account_id: &AccountId) -> u64 {
        env::storage_usage()
    }

    fn after_burn(
        contract: &mut Self,
        _amount: u128,
        _account_id: &AccountId,
        storage_usage_start: u64,
    ) {
        contract.storage_accounting(storage_usage_start);
    }
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }

    pub fn mint(&mut self, amount: U128) {
        Nep141Controller::mint(self, env::predecessor_account_id(), amount.into(), None).unwrap();
    }

    fn require_registration(&self, account_id: &AccountId) {
        require!(
            self.get_storage_balance(account_id).is_some(),
            format!("Account {account_id} is not registered."),
        );
    }

    fn storage_accounting(&mut self, storage_usage_start: u64) {
        let current_usage = env::storage_usage();

        match current_usage.cmp(&storage_usage_start) {
            Ordering::Equal => {}
            Ordering::Greater => {
                let storage_usage = current_usage - storage_usage_start;
                let storage_fee = env::storage_byte_cost() * storage_usage as u128;

                Nep145Controller::lock_storage(
                    self,
                    &env::predecessor_account_id(),
                    storage_fee.into(),
                )
                .unwrap_or_else(|e| env::panic_str(&format!("Storage lock error: {}", e)));
            }
            Ordering::Less => {
                let storage_released = storage_usage_start - current_usage;
                let storage_credit = env::storage_byte_cost() * storage_released as u128;

                Nep145Controller::unlock_storage(
                    self,
                    &env::predecessor_account_id(),
                    storage_credit.into(),
                )
                .unwrap_or_else(|e| env::panic_str(&format!("Storage unlock error: {}", e)));
            }
        }
    }
}
