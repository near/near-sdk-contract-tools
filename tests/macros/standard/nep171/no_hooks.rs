use near_sdk::{PanicOnDefault, env, near};
use near_sdk_contract_tools::nft::*;

#[derive(Nep171, PanicOnDefault)]
#[near(contract_state)]
pub struct Contract {
    pub next_token_id: u32,
}

#[near]
impl Contract {
    pub fn mint(&mut self) -> TokenId {
        let token_id = format!("token_{}", self.next_token_id);
        self.next_token_id += 1;

        Nep171Controller::mint(
            self,
            &Nep171Mint::new(vec![token_id.clone()], env::predecessor_account_id()),
        )
        .unwrap_or_else(|e| env::panic_str(&format!("Minting failed: {e}")));

        token_id
    }
}
