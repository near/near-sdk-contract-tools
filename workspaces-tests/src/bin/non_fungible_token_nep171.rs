#![allow(missing_docs)]

// Ignore
pub fn main() {}

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, log, near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::{hook::Hook, standard::nep171::*, Nep171};

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, Nep171)]
#[nep171(transfer_hook = "Self")]
#[near_bindgen]
pub struct Contract {}

impl Hook<Contract, Nep171Transfer<'_>> for Contract {
    type State = ();

    fn before(_contract: &Contract, transfer: &Nep171Transfer<'_>, _: &mut ()) {
        log!("before_nft_transfer({})", transfer.token_id);
    }

    fn after(_contract: &mut Contract, transfer: &Nep171Transfer<'_>, _: ()) {
        log!("after_nft_transfer({})", transfer.token_id);
    }
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }

    pub fn mint(&mut self, token_ids: Vec<TokenId>) {
        let action = Nep171Mint {
            token_ids: &token_ids,
            receiver_id: &env::predecessor_account_id(),
            memo: None,
        };
        Nep171Controller::mint(self, &action)
            .unwrap_or_else(|e| env::panic_str(&format!("Failed to mint: {:#?}", e)));
    }
}
