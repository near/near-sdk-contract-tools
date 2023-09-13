#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use near_sdk::IntoStorageKey;
pub use near_sdk_contract_tools_macros::*;

/// Default storage keys used by various traits' `root()` functions.
#[derive(Clone, Debug)]
pub enum DefaultStorageKey {
    /// Default storage key for [`approval::ApprovalManagerInternal::root`].
    ApprovalManager,
    /// Default storage key for [`standard::nep141::Nep141ControllerInternal::root`].
    Nep141,
    /// Default storage key for [`standard::nep171::Nep171ControllerInternal::root`].
    Nep171,
    /// Default storage key for [`standard::nep177::Nep177ControllerInternal::root`].
    Nep177,
    /// Default storage key for [`standard::nep178::Nep178ControllerInternal::root`].
    Nep178,
    /// Default storage key for [`standard::nep181::Nep181ControllerInternal::root`].
    Nep181,
    /// Default storage key for [`owner::OwnerInternal::root`].
    Owner,
    /// Default storage key for [`pause::PauseInternal::root`].
    Pause,
    /// Default storage key for [`rbac::RbacInternal::root`].
    Rbac,
    /// Default storage key for [`escrow::Escrow::root`]
    Escrow,
}

impl IntoStorageKey for DefaultStorageKey {
    fn into_storage_key(self) -> Vec<u8> {
        match self {
            DefaultStorageKey::ApprovalManager => b"~am".to_vec(),
            DefaultStorageKey::Nep141 => b"~$141".to_vec(),
            DefaultStorageKey::Nep171 => b"~$171".to_vec(),
            DefaultStorageKey::Nep177 => b"~$177".to_vec(),
            DefaultStorageKey::Nep178 => b"~$178".to_vec(),
            DefaultStorageKey::Nep181 => b"~$181".to_vec(),
            DefaultStorageKey::Owner => b"~o".to_vec(),
            DefaultStorageKey::Pause => b"~p".to_vec(),
            DefaultStorageKey::Rbac => b"~r".to_vec(),
            DefaultStorageKey::Escrow => b"~es".to_vec(),
        }
    }
}

pub mod standard;

pub mod approval;
pub mod escrow;
pub mod fast_account_id;
pub mod migrate;
pub mod owner;
pub mod pause;
pub mod rbac;
pub mod slot;
pub mod upgrade;
pub mod utils;

/// Re-exports of the NFT standard traits.
pub mod nft {
    pub use crate::{
        standard::{nep171::*, nep177::*, nep178::*, nep181::*},
        Nep171, Nep177, Nep178, Nep181, NonFungibleToken,
    };
}
