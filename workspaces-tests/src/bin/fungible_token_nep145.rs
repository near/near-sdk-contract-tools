#![allow(missing_docs)]

// Ignore
pub fn main() {}

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::{Base64VecU8, U128},
    near_bindgen,
    store::Vector,
    AccountId, PanicOnDefault,
};
use near_sdk_contract_tools::{ft::*, standard::nep145::*, utils::Hook, Nep145};

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, FungibleToken, Nep145)]
#[fungible_token(
    all_hooks = "PredecessorStorageAccounting",
    transfer_hook = "TransferHook"
)]
#[near_bindgen]
pub struct Contract {
    blobs: Vector<Vec<u8>>,
}

impl Nep145Hook for Contract {
    fn after_force_unregister(
        contract: &mut Self,
        account_id: &AccountId,
        _balance: &StorageBalance,
    ) {
        let balance = contract.balance_of(account_id);
        contract
            .burn(&Nep141Burn {
                amount: balance,
                account_id: account_id.clone(),
                memo: Some("storage force unregister".to_string()),
            })
            .unwrap();
    }
}

pub struct PredecessorStorageAccounting(u64);

impl<A> Hook<Contract, A> for PredecessorStorageAccounting {
    fn before(contract: &Contract, _args: &A) -> Self {
        contract.require_registration(&env::predecessor_account_id());
        Self(env::storage_usage())
    }

    fn after(contract: &mut Contract, _args: &A, Self(storage_usage_start): Self) {
        contract
            .storage_accounting(&env::predecessor_account_id(), storage_usage_start)
            .unwrap_or_else(|e| env::panic_str(&e.to_string()));
    }
}

pub struct TransferHook(u64);

impl Hook<Contract, Nep141Transfer> for TransferHook {
    fn before(contract: &Contract, transfer: &Nep141Transfer) -> Self {
        contract.require_registration(&transfer.receiver_id);
        Self(env::storage_usage())
    }

    fn after(contract: &mut Contract, _transfer: &Nep141Transfer, Self(storage_usage_start): Self) {
        contract
            .storage_accounting(&env::predecessor_account_id(), storage_usage_start)
            .unwrap_or_else(|e| env::panic_str(&e.to_string()));
    }
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self {
            blobs: Vector::new(b"b"),
        };

        contract.set_metadata(&FungibleTokenMetadata::new(
            "My Fungible Token".to_string(),
            "MFT".to_string(),
            24,
        ));

        contract
    }

    pub fn mint(&mut self, amount: U128) {
        Nep141Controller::mint(
            self,
            &Nep141Mint {
                amount: amount.into(),
                account_id: env::predecessor_account_id(),
                memo: None,
            },
        )
        .unwrap();
    }

    pub fn use_storage(&mut self, blob: Base64VecU8) {
        let storage_start = env::storage_usage();
        let blob = blob.into();
        self.blobs.push(blob);
        self.blobs.flush();
        let storage_end = env::storage_usage();
        self.lock_storage(
            &env::predecessor_account_id(),
            ((storage_end - storage_start) as u128 * env::storage_byte_cost()).into(),
        )
        .unwrap_or_else(|e| env::panic_str(&format!("Storage lock error: {}", e)));
    }

    fn require_registration(&self, account_id: &AccountId) {
        self.get_storage_balance(account_id)
            .unwrap_or_else(|e| env::panic_str(&e.to_string()));
    }
}
