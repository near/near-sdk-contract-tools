#![allow(missing_docs)]

use near_contract_tools::{
    owner::Owner, owner::OwnerExternal, upgrade::serialized::UpgradeHook, Owner, Rbac, Upgrade,
};

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen, PanicOnDefault,
};
pub fn main() {}

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Owner, Upgrade)]
#[upgrade(serializer = "borsh", hook = "owner")]
#[near_bindgen]
pub struct ContractOld {
    pub foo: u32,
}

#[near_bindgen]
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
