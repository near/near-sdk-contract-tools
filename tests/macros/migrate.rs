use near_contract_tools::{
    migrate::{MigrateExternal, MigrateHook},
    Migrate,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen,
};

#[derive(BorshDeserialize, BorshSerialize, Debug)]
#[near_bindgen]
struct Old {
    pub foo: u64,
}

#[near_bindgen]
impl Old {
    #[init]
    pub fn new(foo: u64) -> Self {
        Self { foo }
    }
}

#[derive(Migrate, BorshSerialize, BorshDeserialize)]
#[migrate(from = "Old")]
#[near_bindgen]
struct MyContract {
    pub bar: u64,
}

impl MigrateHook for MyContract {
    fn on_migrate(old: Old) -> Self {
        Self { bar: old.foo }
    }
}

#[test]
fn default_from() {
    let old = Old::new(99);

    // This is done automatically in real #[near_bindgen] WASM contracts
    env::state_write(&old);

    assert_eq!(old.foo, 99);

    let migrated = <MyContract as MigrateExternal>::migrate();

    assert_eq!(migrated.bar, 99);
}
