#![doc = include_str!("../README.md")]

/// Default storage keys used by various traits' `root()` functions.
#[derive(Clone, Debug)]
pub enum DefaultStorageKey {
    /// Default storage key for [`ApprovalManager`]
    ApprovalManager,
    /// Default storage key for [`Nep141Controller`]
    Nep141,
    /// Default storage key for [`Owner`]
    Owner,
    /// Default storage key for [`Pause`]
    Pause,
    /// Default storage key for [`Rbac`]
    Rbac,
}

impl IntoStorageKey for DefaultStorageKey {
    fn into_storage_key(self) -> Vec<u8> {
        match self {
            DefaultStorageKey::ApprovalManager => b"~am".to_vec(),
            DefaultStorageKey::Nep141 => b"~$141".to_vec(),
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

pub use near_contract_tools_macros::*;
use near_sdk::IntoStorageKey;
