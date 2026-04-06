workspaces_tests::predicate!();

use near_sdk::{PanicOnDefault, env, near};
use near_sdk_contract_tools::{Owner, Upgrade, owner::*};

#[derive(Owner, Upgrade, PanicOnDefault)]
#[upgrade(serializer = "borsh", hook = "owner")]
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
