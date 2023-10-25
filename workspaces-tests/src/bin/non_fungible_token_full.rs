#![allow(missing_docs)]

// Ignore
pub fn main() {}

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, log, near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::{
    hook::Hook,
    nft::*,
    standard::nep178::{Nep178Approve, Nep178Revoke, Nep178RevokeAll},
};

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, NonFungibleToken)]
#[near_bindgen]
pub struct Contract {}

impl Hook<Contract, Nep178Approve<'_>> for Contract {
    fn hook<R>(
        contract: &mut Contract,
        args: &Nep178Approve<'_>,
        f: impl FnOnce(&mut Contract) -> R,
    ) -> R {
        log!("before_nft_approve({})", args.token_id);
        let r = f(contract);
        log!("after_nft_approve({})", args.token_id);
        r
    }
}

impl Hook<Contract, Nep178Revoke<'_>> for Contract {
    fn hook<R>(
        contract: &mut Contract,
        args: &Nep178Revoke<'_>,
        f: impl FnOnce(&mut Contract) -> R,
    ) -> R {
        log!("before_nft_revoke({})", args.token_id);
        let r = f(contract);
        log!("after_nft_revoke({})", args.token_id);
        r
    }
}

impl Hook<Contract, Nep178RevokeAll<'_>> for Contract {
    fn hook<R>(
        contract: &mut Contract,
        args: &Nep178RevokeAll<'_>,
        f: impl FnOnce(&mut Contract) -> R,
    ) -> R {
        log!("before_nft_revoke_all({})", args.token_id);
        let r = f(contract);
        log!("after_nft_revoke_all({})", args.token_id);
        r
    }
}

impl Hook<Contract, Nep171Transfer<'_>> for Contract {
    fn hook<R>(
        contract: &mut Contract,
        args: &Nep171Transfer<'_>,
        f: impl FnOnce(&mut Contract) -> R,
    ) -> R {
        log!("before_nft_transfer({})", args.token_id);
        let r = f(contract);
        log!("after_nft_transfer({})", args.token_id);
        r
    }
}

#[near_sdk::near_bindgen]
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
