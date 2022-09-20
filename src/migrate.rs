//! Migrate default struct between two schemas
#![allow(missing_docs)] // #[ext_contract(...)] does not play nicely with clippy

use near_sdk::{
    borsh::{BorshDeserialize, BorshSerialize},
    env, ext_contract,
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
}

/// Called on migration. Must be implemented by the user. (The derive macro
/// does not implement this for you.)
pub trait MigrateHook: MigrateController {
    /// Receives the old schema deserialized from storage as well as optional
    /// arguments from caller, and replaces it with the new schema.
    fn on_migrate(
        old_schema: <Self as MigrateController>::OldSchema,
        args: Option<String>,
    ) -> <Self as MigrateController>::NewSchema;
}

/// Migrate-able contracts expose this trait publically
#[ext_contract(ext_migrate)]
pub trait MigrateExternal {
    /// Perform the migration with optional arguments
    fn migrate(args: Option<String>) -> Self;
}
