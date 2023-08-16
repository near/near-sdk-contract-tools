#![allow(missing_docs)]

// Ignore
pub fn main() {}

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, log, near_bindgen, store, AccountId, PanicOnDefault,
};
use near_sdk_contract_tools::{
    standard::{nep171::*, nep177::*, nep178::*},
    NonFungibleToken,
};

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, NonFungibleToken)]
#[near_bindgen]
pub struct Contract {
    dummy: store::UnorderedMap<String, String>,
}

impl Nep178Hook for Contract {
    fn before_nft_approve(&self, token_id: &TokenId, _account_id: &AccountId) {
        log!("before_nft_approve({})", token_id);
    }

    fn after_nft_approve(
        &mut self,
        token_id: &TokenId,
        _account_id: &AccountId,
        _approval_id: &ApprovalId,
        _state: (),
    ) {
        log!("after_nft_approve({})", token_id);
    }

    fn before_nft_revoke(&self, token_id: &TokenId, _account_id: &AccountId) {
        log!("before_nft_revoke({})", token_id);
    }

    fn after_nft_revoke(&mut self, token_id: &TokenId, _account_id: &AccountId, _state: ()) {
        log!("after_nft_revoke({})", token_id);
    }

    fn before_nft_revoke_all(&self, token_id: &TokenId) {
        log!("before_nft_revoke_all({})", token_id);
    }

    fn after_nft_revoke_all(&mut self, token_id: &TokenId, _state: ()) {
        log!("after_nft_revoke_all({})", token_id);
    }
}

impl Nep171Hook for Contract {
    fn before_nft_transfer(_contract: &Self, transfer: &Nep171Transfer) {
        log!("before_nft_transfer({})", transfer.token_id);
    }

    fn after_nft_transfer(_contract: &mut Self, transfer: &Nep171Transfer, _state: ()) {
        log!("after_nft_transfer({})", transfer.token_id);
    }
}

#[near_sdk::near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self {
            dummy: store::UnorderedMap::new(b"z"),
        };

        contract.set_contract_metadata(ContractMetadata::new(
            "My NFT Smart Contract".to_string(),
            "MNSC".to_string(),
            None,
        ));

        contract
    }

    // TODO: Remove
    pub fn dummy_insert(&mut self) {
        let i = self.dummy.len();
        self.dummy.insert(i.to_string(), format!("value {}", i));
    }

    pub fn dummy_clear(&mut self) {
        self.dummy.clear();
    }

    pub fn dummy_iter(&self) -> Vec<(&String, &String)> {
        self.dummy.into_iter().collect()
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
