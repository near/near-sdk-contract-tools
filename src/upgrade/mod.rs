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
//! By default, after updating the contract logic the contract state is
//! migrated. This behaviour can be changed by providing a
//! custom [`PostUpgrade`].
//!
//! The
#![cfg_attr(feature = "unstable", doc = "[`raw`]")]
#![cfg_attr(not(feature = "unstable"), doc = "`raw` (feature: `unstable`)")]
//! module is included mostly for legacy / compatibility reasons,
//! and for the niche efficiency use-case, since it allows for the most
//! efficient binary serialization (though only by a little). However, it is
//! more difficult to use and has more sharp edges.
//!
//! # Safety
//!
//! If the contract state is migrated, the new contract logic must deserialize
//! the existing state according to the old schema and migrate it to the new
//! schema. If the new contract has a different storage schema from the old
//! contract and does not migrate the state schema, the contract may become
//! unusable.
use near_sdk::Gas;

/// Default value for the name of the function that will be called after
/// upgrade (usually a migrate function).
pub const DEFAULT_POST_UPGRADE_METHOD_NAME: &str = "migrate";
/// Default input to send to the post-upgrade function.
pub const DEFAULT_POST_UPGRADE_METHOD_ARGS: Vec<u8> = vec![];
/// Guarantee the post-upgrade function receives at least this much gas by
/// default.
#[cfg(feature = "near-sdk-4")]
pub const DEFAULT_POST_UPGRADE_MINIMUM_GAS: Gas = Gas(15_000_000_000_000);
/// Guarantee the post-upgrade function receives at least this much gas by
/// default.
#[cfg(feature = "near-sdk-5")]
pub const DEFAULT_POST_UPGRADE_MINIMUM_GAS: Gas = Gas::from_gas(15_000_000_000_000);

#[cfg(feature = "unstable")]
pub mod raw;
pub mod serialized;

/// Function call after upgrade descriptor
#[derive(Debug, Clone)]
pub struct PostUpgrade {
    /// Function name
    pub method: String,
    /// Serialized function input
    pub args: Vec<u8>,
    /// Guarantee minimum gas
    pub minimum_gas: Gas,
}

impl Default for PostUpgrade {
    fn default() -> Self {
        Self {
            method: DEFAULT_POST_UPGRADE_METHOD_NAME.to_string(),
            args: DEFAULT_POST_UPGRADE_METHOD_ARGS,
            minimum_gas: DEFAULT_POST_UPGRADE_MINIMUM_GAS,
        }
    }
}
