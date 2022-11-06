//! Contract code upgrades.
//!
//! If you ever wish to update the logic of your contract without having to
//! use a full-access key, you can take advantage of the upgrade patterns in
//! this module. Upgrading a contract usually consists of two parts:
//!
//! 1. A reflexive `DEPLOY_CONTRACT` action.
//! 2. A follow-up state migration, if necessary. (See: [`crate::migrate`].)
//!
//! This module provides a few different ways to perform an upgrade. Most new
//! projects should probably start with the [`serialized`] module.
//!
//! The [`raw`] module is included mostly for legacy / compatibility reasons,
//! and for the niche efficiency use-case, since it allows for the most
//! efficient binary serialization (though only by a little). However, it is
//! more difficult to use and has more sharp edges.
use near_sdk::Gas;

/// The function that will be called after upgrade (usually a migrate function)
pub const DEFAULT_MIGRATE_METHOD_NAME: &str = "migrate";
/// Input to send to the function called after upgrade
pub const DEFAULT_MIGRATE_METHOD_ARGS: Vec<u8> = vec![];
/// Guarantee the migrate function receives at least this much gas
pub const DEFAULT_MIGRATE_MINIMUM_GAS: Gas = Gas(15_000_000_000_000);

pub mod raw;
pub mod serialized;
