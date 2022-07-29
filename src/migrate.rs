use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env, ext_contract,
};

pub trait MigrateController {
    type OldSchema: BorshDeserialize;
    type NewSchema: From<Self::OldSchema> + BorshSerialize;

    fn convert_state() -> Self::NewSchema {
        let old_state = env::state_read::<Self::OldSchema>()
            .unwrap_or_else(|| env::panic_str("Failed to deserialize old state"));

        let new_state = Self::NewSchema::from(old_state);

        new_state
    }
}
