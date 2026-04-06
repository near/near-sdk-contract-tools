workspaces_tests::predicate!();

use near_sdk::{PanicOnDefault, env, near};
use near_sdk_contract_tools::{Owner, owner::*, upgrade::PostUpgrade};

#[derive(Owner, PanicOnDefault)]
#[near(contract_state)]
pub struct ContractOld {
    pub foo: u32,
}

#[near]
impl ContractOld {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self { foo: 0 };

        Owner::init(&mut contract, &env::predecessor_account_id());
        contract
    }

    pub fn increment_foo(&mut self) {
        self.foo += 1;
    }

    pub fn get_foo(&self) -> u32 {
        self.foo
    }
}

#[unsafe(no_mangle)]
pub fn upgrade() {
    near_sdk::env::setup_panic_hook();

    ContractOld::require_owner();

    unsafe {
        near_sdk_contract_tools::upgrade::raw::upgrade(PostUpgrade::default());
    }
}
