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
    /// Default storage key for [`standard::nep145::Nep145ControllerInternal::root`]
    Nep145,
    /// Default storage key for [`standard::nep148::Nep148ControllerInternal::root`].
    Nep148,
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
    /// Default storage key for [`escrow::EscrowInternal::root`]
    Escrow,
}

impl IntoStorageKey for DefaultStorageKey {
    fn into_storage_key(self) -> Vec<u8> {
        match self {
            DefaultStorageKey::ApprovalManager => b"~am".to_vec(),
            DefaultStorageKey::Nep141 => b"~$141".to_vec(),
            DefaultStorageKey::Nep145 => b"~$145".to_vec(),
            DefaultStorageKey::Nep148 => b"~$148".to_vec(),
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
pub mod hook;
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
        standard::{
            nep145::{
                self, ext_nep145, Nep145, Nep145Controller, Nep145ControllerInternal,
                StorageBalance, StorageBalanceBounds,
            },
            nep171::{
                self, ext_nep171, ext_nep171_receiver, ext_nep171_resolver, Nep171,
                Nep171Controller, Nep171ControllerInternal, Nep171Hook, Nep171Receiver,
                Nep171Resolver, Nep171Transfer, SimpleNep171Hook, Token, TokenId,
            },
            nep177::{
                self, ext_nep177, ContractMetadata, Nep177, Nep177Controller,
                Nep177ControllerInternal, TokenMetadata,
            },
            nep178::{
                self, ext_nep178, ext_nep178_receiver, ApprovalId, Nep178, Nep178Controller,
                Nep178ControllerInternal, Nep178Hook, Nep178Receiver, SimpleNep178Hook,
                TokenApprovals,
            },
            nep181::{
                self, ext_nep181, Nep181, Nep181Controller, Nep181ControllerInternal,
                TokenEnumeration,
            },
        },
        Nep171, Nep177, Nep178, Nep181, NonFungibleToken,
    };
}

/// Re-exports of the FT standard traits.
pub mod ft {
    pub use crate::{
        standard::{
            nep141::{
                self, ext_nep141, ext_nep141_receiver, ext_nep141_resolver, Nep141, Nep141Burn,
                Nep141Controller, Nep141ControllerInternal, Nep141Mint, Nep141Receiver,
                Nep141Resolver, Nep141Transfer,
            },
            nep145::{
                self, ext_nep145, Nep145, Nep145Controller, Nep145ControllerInternal,
                StorageBalance, StorageBalanceBounds,
            },
            nep148::{
                self, ext_nep148, FungibleTokenMetadata, Nep148, Nep148Controller,
                Nep148ControllerInternal,
            },
        },
        FungibleToken, Nep141, Nep145, Nep148,
    };
}
