use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::{
    owner::Owner,
    pause::Pause,
    standard::{nep171::*, nep177::*},
    NonFungibleToken, Owner, Pause,
};

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, NonFungibleToken, Pause, Owner)]
#[non_fungible_token(no_approval_hooks)]
#[near_bindgen]
pub struct Contract {
    next_token_id: u32,
}

impl Nep171Hook for Contract {
    type NftTransferState = ();

    fn before_nft_transfer(_contract: &Self, _transfer: &Nep171Transfer) {
        Self::require_unpaused();
    }

    fn after_nft_transfer(_contract: &mut Self, _transfer: &Nep171Transfer, _: ()) {}
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self { next_token_id: 0 };

        contract.set_contract_metadata(ContractMetadata::new(
            "My NFT".to_string(),
            "MYNFT".to_string(),
            None,
        ));

        Owner::init(&mut contract, &env::predecessor_account_id());

        contract
    }

    pub fn mint(&mut self) -> TokenId {
        Self::require_unpaused();

        let token_id = format!("token_{}", self.next_token_id);
        self.next_token_id += 1;
        self.mint_with_metadata(
            token_id.clone(),
            env::predecessor_account_id(),
            TokenMetadata::new()
                .title(format!("Token {token_id}"))
                .description(format!("This is token {token_id}.")),
        )
        .unwrap_or_else(|e| env::panic_str(&format!("Minting failed: {e}")));

        token_id
    }
}
