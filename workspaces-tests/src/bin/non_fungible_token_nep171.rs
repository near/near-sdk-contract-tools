workspaces_tests::predicate!();

use near_sdk::{PanicOnDefault, env, log, near};
use near_sdk_contract_tools::{Nep171, hook::Hook, nft::Nep171Mint, standard::nep171::*};

#[derive(Nep171, PanicOnDefault)]
#[nep171(transfer_hook = "Self")]
#[near(contract_state)]
pub struct Contract {}

impl Hook<Contract, action::Nep171Transfer<'_>> for Contract {
    fn hook<R>(
        contract: &mut Contract,
        args: &action::Nep171Transfer<'_>,
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
        Self {}
    }

    pub fn mint(&mut self, token_ids: Vec<TokenId>) {
        Nep171Controller::mint(
            self,
            &Nep171Mint::new(token_ids, env::predecessor_account_id()),
        )
        .unwrap_or_else(|e| env::panic_str(&format!("Failed to mint: {:#?}", e)));
    }
}
