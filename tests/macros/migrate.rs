use near_contract_tools::Migrate;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen,
};
use serde::Deserialize;

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

fn none() {}

#[derive(Migrate, BorshSerialize, BorshDeserialize)]
#[migrate(from = "Old", on_migrate = "none")]
#[near_bindgen]
struct NewDefaultFrom {
    pub bar: u64,
}

impl From<Old> for NewDefaultFrom {
    fn from(old: Old) -> Self {
        Self { bar: old.foo }
    }
}

#[derive(Migrate, BorshSerialize, BorshDeserialize)]
#[migrate(from = "Old", on_migrate = "none", convert = "custom_convert_no_args")]
#[near_bindgen]
struct NewNoArgs {
    pub bar: u64,
}

fn custom_convert_no_args(old: Old) -> NewNoArgs {
    near_sdk::log!("custom_convert_no_args");
    NewNoArgs { bar: old.foo }
}

#[derive(Migrate, BorshSerialize, BorshDeserialize)]
#[migrate(
    from = "Old",
    on_migrate = "none",
    convert_with_args = "custom_convert_with_args"
)]
#[near_bindgen]
struct NewWithArgs {
    pub bar: u64,
}

fn custom_convert_with_args(old: Old, args: String) -> NewWithArgs {
    #[derive(Debug, Deserialize)]
    struct CustomArgs {
        pub add: u64,
    }

    let args: CustomArgs = serde_json::from_str(&args).unwrap();

    near_sdk::log!(format!("custom_convert_with_args: {args:?}"));
    NewWithArgs {
        bar: old.foo + args.add,
    }
}

#[test]
fn default_from() {
    let old = Old::new(99);

    // This is done automatically in real #[near_bindgen] WASM contracts
    env::state_write(&old);

    assert_eq!(old.foo, 99);

    let migrated = NewDefaultFrom::migrate();

    assert_eq!(migrated.bar, 99);
}

#[test]
fn no_args() {
    let old = Old::new(99);
    env::state_write(&old);

    assert_eq!(old.foo, 99);

    let migrated = NewNoArgs::migrate();

    assert_eq!(migrated.bar, 99);
}

#[test]
fn with_args() {
    let old = Old::new(99);
    env::state_write(&old);

    assert_eq!(old.foo, 99);

    let migrated = NewWithArgs::migrate(r#"{"add":1}"#.to_string());

    assert_eq!(migrated.bar, 100);
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
