workspaces_tests::predicate!();

use near_sdk::{PanicOnDefault, near};

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct ContractBad {
    pub foo: u32,
}

#[near]
impl ContractBad {
    #[init]
    pub fn new() -> Self {
        Self { foo: 0 }
    }

    pub fn increment_foo(&mut self) {
        self.foo += 1;
    }

    pub fn get_foo(&self) -> u32 {
        self.foo
    }
}
