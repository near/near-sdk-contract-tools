#![allow(missing_docs)]

// Ignore
pub fn main() {}

workspaces_tests::near_sdk!();
compat_use_borsh!();
use near_sdk::{
    env,
    json_types::{Base64VecU8, U128},
    near_bindgen,
    store::Vector,
    PanicOnDefault,
};
use near_sdk_contract_tools::{compat_derive_borsh, compat_near_to_u128, compat_use_borsh, ft::*};

compat_derive_borsh! {
    #[derive(PanicOnDefault, FungibleToken)]
    #[near_bindgen]
    pub struct Contract {
        blobs: Vector<Vec<u8>>,
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
                receiver_id: &env::predecessor_account_id(),
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
            ((storage_end - storage_start) as u128
                * compat_near_to_u128!(env::storage_byte_cost()))
            .into(),
        )
        .unwrap_or_else(|e| env::panic_str(&format!("Storage lock error: {}", e)));
    }
}
