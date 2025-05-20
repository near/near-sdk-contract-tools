workspaces_tests::predicate!();

use near_sdk::{env, json_types::U128, near};
use near_sdk_contract_tools::mt::*;

#[derive(Default, Nep245)]
#[near(contract_state)]
pub struct Contract {}

#[near]
impl Contract {
    pub fn mint(&mut self, token_id: String, amount: U128) {
        if self.token(&token_id).is_none() {
            self.create_token(token_id.clone()).unwrap();
        }
        Nep245Controller::mint(
            self,
            &Nep245Mint::single(token_id, amount.0, env::predecessor_account_id()),
        )
        .unwrap_or_else(|e| env::panic_str(&e.to_string()));
    }
}
