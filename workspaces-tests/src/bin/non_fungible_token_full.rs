#![allow(missing_docs)]

// Ignore
pub fn main() {}

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, log, near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::{
    standard::{nep171::*, nep177::*},
    NonFungibleToken,
};

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, NonFungibleToken)]
#[near_bindgen]
pub struct Contract {}

impl Nep171Hook for Contract {
    fn before_nft_transfer(_contract: &Self, transfer: &Nep171Transfer) {
        log!("before_nft_transfer({})", transfer.token_id);
    }

    fn after_nft_transfer(_contract: &mut Self, transfer: &Nep171Transfer, _state: ()) {
        log!("after_nft_transfer({})", transfer.token_id);
    }
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self {};

        contract.set_contract_metadata(ContractMetadata::new(
            "My NFT Smart Contract".to_string(),
            "MNSC".to_string(),
            None,
        ));

        contract
    }

    pub fn mint(&mut self, token_ids: Vec<TokenId>) {
        let receiver = env::predecessor_account_id();
        for token_id in token_ids {
            self.mint_with_metadata(
                token_id.clone(),
                receiver.clone(),
                TokenMetadata {
                    title: Some(token_id),
                    description: Some("description".to_string()),
                    media: None,
                    media_hash: None,
                    copies: None,
                    issued_at: None,
                    expires_at: None,
                    starts_at: None,
                    updated_at: None,
                    extra: None,
                    reference: None,
                    reference_hash: None,
                },
            )
            .unwrap_or_else(|e| env::panic_str(&format!("Failed to mint: {:#?}", e)));
        }
    }
}
