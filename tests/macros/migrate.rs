use near_contract_tools::Migrate;
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
struct NewNoArgs {
    pub bar: u64,
}

impl From<Old> for NewNoArgs {
    fn from(old: Old) -> Self {
        Self { bar: old.foo }
    }
}

#[derive(Migrate, BorshSerialize, BorshDeserialize)]
#[migrate(from = "Old", hook = "NewWithArgs::migrate_hook", args = "String")]
#[near_bindgen]
struct NewWithArgs {
    pub bar: u64,
}

impl NewWithArgs {
    pub fn migrate_hook(args: String) {
        println!("migrate_hook: {args}");
    }
}

impl From<Old> for NewWithArgs {
    fn from(old: Old) -> Self {
        Self { bar: old.foo }
    }
}

#[test]
fn no_args() {
    let old = Old::new(99);

    // This is done automatically in real #[near_bindgen] WASM contracts
    env::state_write(&old);

    assert_eq!(old.foo, 99);

    let migrated = NewNoArgs::migrate();

    assert_eq!(migrated.bar, 99);
}

// impl near_contract_tools::migrate::MigrateController for Contract {
//     type OldState = Old;
//     type NewState = New;
// }

// #[near_bindgen]
// impl near_contract_tools::migrate::Migrate for Contract {
//     #[private]
//     #[init(ignore_state)]
//     fn migrate() -> Vec<u8> {
//         <Contract as near_contract_tools::migrate::MigrateController>::convert_state()
//             .try_to_vec()
//             .unwrap()
//     }
// }
