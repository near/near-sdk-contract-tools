use near_sdk::{
    PanicOnDefault, env,
    json_types::{Base64VecU8, U128},
    near,
    store::Vector,
};
use near_sdk_contract_tools::ft::*;

#[derive(FungibleToken, PanicOnDefault)]
#[near(contract_state)]
pub struct Contract {
    blobs: Vector<Vec<u8>>,
}

#[near]
impl Contract {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self {
            blobs: Vector::new(b"b"),
        };

        contract.set_metadata(&ContractMetadata::new("My Fungible Token", "MYFT", 24));

        contract
    }

    pub fn mint(&mut self, amount: U128) {
        Nep141Controller::mint(
            self,
            &Nep141Mint::new(amount.0, env::predecessor_account_id()),
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
            env::storage_byte_cost().saturating_mul(u128::from(storage_end - storage_start)),
        )
        .unwrap_or_else(|e| env::panic_str(&format!("Storage lock error: {}", e)));
    }
}
