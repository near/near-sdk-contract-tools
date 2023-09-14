use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::nft::*;

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Nep171)]
#[nep171(no_hooks)]
#[near_bindgen]
pub struct Contract {
    pub next_token_id: u32,
}

#[near_bindgen]
impl Contract {
    pub fn mint(&mut self) -> TokenId {
        let token_id = format!("token_{}", self.next_token_id);
        self.next_token_id += 1;
        Nep171Controller::mint(
            self,
            &[token_id.clone()],
            &env::predecessor_account_id(),
            None,
        )
        .unwrap_or_else(|e| env::panic_str(&format!("Minting failed: {e}")));
        token_id
    }
}
