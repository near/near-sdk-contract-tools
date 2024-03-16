#![allow(missing_docs)]

// Ignore
pub fn main() {}

workspaces_tests::near_sdk!();
compat_use_borsh!();
use near_sdk::{env, log, near_bindgen, PanicOnDefault};
use near_sdk_contract_tools::{compat_derive_borsh, compat_use_borsh, hook::Hook, nft::*};

compat_derive_borsh! {
    #[derive(PanicOnDefault, NonFungibleToken)]
    #[near_bindgen]
    pub struct Contract {}
}

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
