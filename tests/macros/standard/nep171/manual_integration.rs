use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::{
    owner::Owner,
    pause::Pause,
    standard::{
        nep171::*,
        nep177::{self, Nep177Controller},
        nep178,
    },
    Nep171, Nep177, Nep178, Owner, Pause,
};

#[derive(
    BorshSerialize, BorshDeserialize, PanicOnDefault, Nep171, Nep177, Nep178, Pause, Owner,
)]
#[nep171(
    extension_hooks = "nep178::TokenApprovals",
    check_external_transfer = "nep178::TokenApprovals",
    token_data = "(nep177::TokenMetadata, nep178::TokenApprovals)"
)]
#[nep178(no_hooks)]
#[near_bindgen]
pub struct Contract {
    next_token_id: u32,
}

impl Nep171Hook for Contract {
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

        contract.set_contract_metadata(nep177::ContractMetadata::new(
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
        Nep177Controller::mint_with_metadata(
            self,
            token_id.clone(),
            env::predecessor_account_id(),
            nep177::TokenMetadata::new()
                .title(format!("Token {token_id}"))
                .description(format!("This is token {token_id}.")),
        )
        .unwrap_or_else(|e| env::panic_str(&format!("Minting failed: {e}")));

        token_id
    }
}
