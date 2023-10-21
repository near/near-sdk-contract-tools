use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::{nft::*, standard::nep171::Nep171Mint};

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Nep171)]
#[near_bindgen]
pub struct Contract {
    pub next_token_id: u32,
}

#[near_bindgen]
impl Contract {
    pub fn mint(&mut self) -> TokenId {
        let token_id = format!("token_{}", self.next_token_id);
        self.next_token_id += 1;

        let token_ids = [token_id];
        let action = Nep171Mint {
            token_ids: &token_ids,
            receiver_id: &env::predecessor_account_id(),
            memo: None,
        };
        Nep171Controller::mint(self, &action)
            .unwrap_or_else(|e| env::panic_str(&format!("Minting failed: {e}")));

        let [token_id] = token_ids;

        token_id
    }
}
