use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env,
};

pub trait MigrateController {
    type OldSchema: BorshDeserialize;
    type NewSchema: BorshSerialize;

    fn deserialize_old_schema() -> Self::OldSchema {
        env::state_read::<Self::OldSchema>()
            .unwrap_or_else(|| env::panic_str("Failed to deserialize old state"))
    }

    fn convert(old_state: Self::OldSchema, _args: Option<String>) -> Self::NewSchema;
}
