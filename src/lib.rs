#![doc = include_str!("../README.md")]

/// Default storage keys used by various traits' `root()` functions.
#[derive(Clone, Debug)]
pub enum DefaultStorageKey {
    /// Default storage key for [`approval::ApprovalManagerInternal::root`]
    ApprovalManager,
    /// Default storage key for [`standard::nep141::Nep141ControllerInternal::root`]
    Nep141,
    /// Default storage key for [`standard::nep145::Nep145ControllerInternal::root`]
    Nep145,
    /// Default storage key for [`owner::OwnerInternal::root`]
    Owner,
    /// Default storage key for [`pause::PauseInternal::root`]
    Pause,
    /// Default storage key for [`rbac::RbacInternal::root`]
    Rbac,
}

impl IntoStorageKey for DefaultStorageKey {
    fn into_storage_key(self) -> Vec<u8> {
        match self {
            DefaultStorageKey::ApprovalManager => b"~am".to_vec(),
            DefaultStorageKey::Nep141 => b"~$141".to_vec(),
            DefaultStorageKey::Nep145 => b"~$145".to_vec(),
            DefaultStorageKey::Owner => b"~o".to_vec(),
            DefaultStorageKey::Pause => b"~p".to_vec(),
            DefaultStorageKey::Rbac => b"~r".to_vec(),
        }
    }
}

pub mod standard;

pub mod approval;
pub mod migrate;
pub mod owner;
pub mod pause;
pub mod rbac;
pub mod slot;
pub mod upgrade;
pub mod utils;

use near_sdk::IntoStorageKey;
pub use near_sdk_contract_tools_macros::*;
