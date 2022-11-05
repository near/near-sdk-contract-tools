use near_sdk::Gas;

pub const DEFAULT_MIGRATE_METHOD_NAME: &str = "migrate";
pub const DEFAULT_MIGRATE_METHOD_ARGS: Vec<u8> = vec![];
pub const DEFAULT_MIGRATE_MINIMUM_GAS: Gas = Gas(15_000_000_000_000);

pub mod raw;
pub mod serialized;
