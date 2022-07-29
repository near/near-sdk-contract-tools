//! Migrate default struct between two schemas

use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env,
};

// TODO: Migration events?
// *Possibly* unnecessary, since the salient occurence will probably be the instigating event (e.g. a code upgrade)
// Alternative solution: post-migration hook/callback so that the author can implement their own events if desired

/// Conversion between two storage schemas
pub trait MigrateController {
    /// Schema that currently exists in storage, to convert from
    type OldSchema: BorshDeserialize;
    /// Schema that will be used henceforth, to convert into
    type NewSchema: BorshSerialize;

    /// Deserializes the old schema from storage.
    /// 
    /// It is probably not necessary to override this function.
    fn deserialize_old_schema() -> Self::OldSchema {
        env::state_read::<Self::OldSchema>()
            .unwrap_or_else(|| env::panic_str("Failed to deserialize old state"))
    }

    /// Convert an old schema to a new schema, with additional arguments
    /// optionally passed as a string.
    fn convert(old_state: Self::OldSchema, _args: Option<String>) -> Self::NewSchema;
}
