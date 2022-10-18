//! Migrate pattern implements methods to change the storage representation of a struct.
//!
//! The migration controller takes the old and new schema and deserializes
//! the contract state from the old schema. The [`on_migrate`][`MigrateHook::on_migrate`]
//! method takes this state and replaces it with the new schema.
//! [`MigrateExternal`] exposes this functionality publicly. This
//! [derive_macro](near_contract_tools_macros::Migrate) derives a default implementation
//! for migration.
//!
//! Note: [`MigrateHook`] must be implemented by the user and is not derived
//! by default. It must convert data in the old schema to the new schema without
//! failing. For a complete example checkout [upgrade_new.rs](https://github.com/NEARFoundation/near-contract-tools/blob/develop/workspaces-tests/src/bin/upgrade_new.rs)
//! in workspace-tests.
//!
//! # Safety
//! The contract state must conform to the old schema otherwise deserializing it
//! will fail and throw an error.
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
    ) -> <Self as MigrateController>::NewSchema;
}

/// Migrate-able contracts expose this trait publicly
#[ext_contract(ext_migrate)]
pub trait MigrateExternal {
    /// Perform the migration with optional arguments
    fn migrate() -> Self;
}
