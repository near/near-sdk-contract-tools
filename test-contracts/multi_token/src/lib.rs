use near_sdk::{
    PanicOnDefault, env,
    json_types::{Base64VecU8, U128},
    near,
    store::Vector,
};
use near_sdk_contract_tools::mt::*;

#[derive(PanicOnDefault, MultiToken)]
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

        contract.set_contract_metadata(&nep245::metadata::ContractMetadata::new("My MultiToken"));

        contract
    }

    pub fn mint(&mut self, token_id: String, amount: U128) {
        if self.token(&token_id).is_none() {
            self.create_token(token_id.clone()).unwrap();
        }
        Nep245Controller::mint(
            self,
            &Nep245Mint::single(token_id, amount.0, env::predecessor_account_id()),
        )
        .unwrap_or_else(|e| env::panic_str(&e.to_string()));
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

    pub fn create_base_meta(&mut self, base_metadata: BaseMetadata) {
        MetadataController::create_base_metadata(self, base_metadata)
            .unwrap_or_else(|e| env::panic_str(&e.to_string()));
    }

    pub fn remove_base_meta(&mut self, base_metadata_id: BaseMetadataId) {
        MetadataController::remove_base_metadata(self, base_metadata_id)
            .unwrap_or_else(|e| env::panic_str(&e.to_string()));
    }

    pub fn set_token_meta(
        &mut self,
        token_id: TokenId,
        base_metadata_id: BaseMetadataId,
        token_metadata: TokenMetadata,
    ) {
        MetadataController::set_token_metadata(self, &token_id, base_metadata_id, token_metadata)
            .unwrap_or_else(|e| env::panic_str(&e.to_string()));
    }
}
