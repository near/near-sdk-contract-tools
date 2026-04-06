workspaces_tests::predicate!();

use near_sdk::{PanicOnDefault, env, log, near, serde_json::json};
use near_sdk_contract_tools::{
    hook::Hook,
    nft::{
        nep171::{CheckExternalTransfer, LoadTokenMetadata},
        *,
    },
};

#[derive(NonFungibleToken, PanicOnDefault)]
#[non_fungible_token(
    transfer_hook = "Self",
    approve_hook = "Self",
    revoke_hook = "Self",
    revoke_all_hook = "Self",
    token_data = "ExtraTokenData",
    check_external_transfer = "ExtraCheckExternalTransfer"
)]
#[near(contract_state)]
pub struct Contract {}

pub struct ExtraCheckExternalTransfer;

impl CheckExternalTransfer<Contract> for ExtraCheckExternalTransfer {
    fn check_external_transfer(
        contract: &Contract,
        transfer: &Nep171Transfer,
    ) -> Result<near_sdk::AccountId, nep171::error::Nep171TransferError> {
        TokenApprovals::check_external_transfer(contract, transfer)
    }
}

pub struct ExtraTokenData;

impl LoadTokenMetadata<Contract> for ExtraTokenData {
    fn load(
        _contract: &Contract,
        _token_id: &TokenId,
        metadata: &mut std::collections::HashMap<String, near_sdk::serde_json::Value>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        metadata.insert(
            "funky_data".to_string(),
            json!({
                "funky": "data",
            }),
        );
        Ok(())
    }
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

#[near]
impl Contract {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self {};

        contract.set_contract_metadata(&ContractMetadata::new(
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
                &token_id,
                &receiver,
                &TokenMetadata::new()
                    .title(token_id.clone())
                    .description("description"),
            )
            .unwrap_or_else(|e| env::panic_str(&format!("Failed to mint: {:#?}", e)));
        }
    }
}
