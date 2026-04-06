use near_sdk::{PanicOnDefault, env, near};
use near_sdk_contract_tools::{Migrate, migrate::MigrateHook};

mod old {
    use super::*;

    #[derive(Debug, PanicOnDefault)]
    #[near(contract_state)]
    pub struct Old {
        pub foo: u64,
    }

    #[near]
    impl Old {
        #[init]
        pub fn new(foo: u64) -> Self {
            Self { foo }
        }
    }
}

#[derive(Migrate, PanicOnDefault)]
#[migrate(from = "old::Old")]
#[near(contract_state)]
struct MyContract {
    pub bar: u64,
}

impl MigrateHook for MyContract {
    fn on_migrate(old: old::Old) -> Self {
        Self { bar: old.foo }
    }
}

#[test]
fn default_from() {
    let old = old::Old::new(99);

    // This is done automatically in real #[near] WASM contracts
    env::state_write(&old);

    assert_eq!(old.foo, 99);

    let migrated = MyContract::migrate();

    assert_eq!(migrated.bar, 99);
}
