#![allow(missing_docs)]

workspaces_tests::predicate!();
use near_sdk::{near_bindgen, PanicOnDefault};
use near_sdk_contract_tools::{compat_derive_borsh, migrate::*, Migrate};

compat_derive_borsh! {[BorshDeserialize],
    pub struct ContractOld {
        pub foo: u32,
    }
}

compat_derive_borsh! {
    #[derive(PanicOnDefault, Migrate)]
    #[migrate(from = "ContractOld")]
    #[near_bindgen]
    pub struct ContractNew {
        pub bar: u64,
    }
}

impl MigrateHook for ContractNew {
    fn on_migrate(old_schema: ContractOld) -> Self {
        Self {
            bar: old_schema.foo as u64,
        }
    }
}

#[near_bindgen]
impl ContractNew {
    #[init]
    pub fn new() -> Self {
        Self { bar: 0 }
    }

    pub fn get_bar(&self) -> u64 {
        self.bar
    }
}
