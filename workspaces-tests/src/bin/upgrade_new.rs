workspaces_tests::predicate!();

use near_sdk::{PanicOnDefault, near};
use near_sdk_contract_tools::{Migrate, migrate::*};

#[near]
pub struct ContractOld {
    pub foo: u32,
}

#[derive(Migrate, PanicOnDefault)]
#[migrate(from = "ContractOld")]
#[near(contract_state)]
pub struct ContractNew {
    pub bar: u64,
}

impl MigrateHook for ContractNew {
    fn on_migrate(old_schema: ContractOld) -> Self {
        Self {
            bar: old_schema.foo as u64,
        }
    }
}

#[near]
impl ContractNew {
    #[init]
    pub fn new() -> Self {
        Self { bar: 0 }
    }

    pub fn get_bar(&self) -> u64 {
        self.bar
    }
}
