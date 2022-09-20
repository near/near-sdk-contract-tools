#![allow(missing_docs)]

use near_contract_tools::upgrade::{upgrade as other_upgrade, Upgrade, UpgradeHook};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    near_bindgen, PanicOnDefault,
};
pub fn main() {}

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
#[near_bindgen]
pub struct ContractOld {
    pub foo: u32,
}

/// ok nice, lets just do this and I will follow you to the end of the world (that was Github Copilot lmao)
// you can enable AUDIO CALL in vscode ok let me try that
// :)
#[near_bindgen]
impl ContractOld {
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

    // wouldn't this be migrate ?
    // wrong upgrade function - #[near_bindgen] will try to deserialize args as JSON
    pub fn upgrade(&self) {
        // do upgrade stuff and return a promise
        // this is the old contract
        <Self as Upgrade>::upgrade()
    }
}

#[no_mangle]
pub fn call_upgrade() {
    other_upgrade::<ContractOld>();
}

impl Upgrade for ContractOld {
    fn upgrade() {
        // do upgrade stuff
        other_upgrade::<ContractOld>();
    }
}
