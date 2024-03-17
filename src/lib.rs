#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

#[cfg(all(feature = "near-sdk-4", feature = "near-sdk-5"))]
compile_error!("Features `near-sdk-4` and `near-sdk-5` cannot be enabled at the same time.");

#[cfg(feature = "near-sdk-4")]
pub extern crate near_sdk_4 as near_sdk;
#[cfg(feature = "near-sdk-5")]
pub extern crate near_sdk_5 as near_sdk;

/// **COMPATIBLE (UNSTABLE)**
///
/// Data structure representing an amount of NEAR tokens. Only for
/// compatibility between `near_sdk` versions.
#[cfg(feature = "near-sdk-4")]
pub type CompatNearToken = near_sdk::Balance;
/// **COMPATIBLE (UNSTABLE)**
///
/// Data structure representing an amount of NEAR tokens. Only for
/// compatibility between `near_sdk` versions.
#[cfg(feature = "near-sdk-5")]
pub type CompatNearToken = near_sdk::NearToken;

/// **COMPATIBLE (UNSTABLE)**
///
/// 1 NEAR token.
pub static COMPAT_ONE_NEAR: Lazy<CompatNearToken> = Lazy::new(|| compat_near!(1u128));

/// **COMPATIBLE (UNSTABLE)**
///
/// 1 yoctoNEAR token.
pub static COMPAT_ONE_YOCTONEAR: Lazy<CompatNearToken> = Lazy::new(|| compat_yoctonear!(1u128));

/// **COMPATIBLE (UNSTABLE)**
///
/// 1 Gas unit.
pub static COMPAT_ONE_GAS: Lazy<near_sdk::Gas> = Lazy::new(|| compat_gas!(1u64));

/// **COMPATIBLE (UNSTABLE)**
///
/// 1 gigaGas unit.
pub static COMPAT_ONE_GIGAGAS: Lazy<near_sdk::Gas> = Lazy::new(|| compat_gas!(10u64.pow(9)));

/// **COMPATIBLE (UNSTABLE)**
///
/// 1 teraGas unit.
pub static COMPAT_ONE_TERAGAS: Lazy<near_sdk::Gas> = Lazy::new(|| compat_gas!(10u64.pow(12)));

/// **COMPATIBLE (UNSTABLE)**
///
/// Converts a number into the NEAR balance amount for the current `near_sdk`
/// version, depending on feature flags. Value is given in units of yoctoNEAR.
#[macro_export]
macro_rules! compat_yoctonear {
    ($amount: expr) => {{
        #[cfg(feature = "near-sdk-4")]
        {
            near_sdk::Balance::from($amount)
        }
        #[cfg(feature = "near-sdk-5")]
        {
            near_sdk::NearToken::from_yoctonear(u128::from($amount))
        }
    }};
}

/// **COMPATIBLE (UNSTABLE)**
///
/// Converts a number into the NEAR balance amount for the current `near_sdk`
/// version, depending on feature flags. Value is given in units of NEAR.
#[macro_export]
macro_rules! compat_near {
    ($amount: expr) => {{
        #[cfg(feature = "near-sdk-4")]
        {
            near_sdk::Balance::from(10u128.pow(24) * $amount)
        }
        #[cfg(feature = "near-sdk-5")]
        {
            near_sdk::NearToken::from_near(u128::from($amount))
        }
    }};
}

/// **COMPATIBLE (UNSTABLE)**
///
/// Converts a NEAR balance amount to a `u128` for the current `near_sdk`
/// version, depending on feature flags.
#[macro_export]
macro_rules! compat_near_to_u128 {
    ($amount: expr) => {{
        #[cfg(feature = "near-sdk-4")]
        {
            u128::from($amount)
        }
        #[cfg(feature = "near-sdk-5")]
        {
            near_sdk::NearToken::as_yoctonear(&$amount)
        }
    }};
}

/// **COMPATIBLE (UNSTABLE)**
///
/// Converts a u64 to a gas value for the current `near_sdk` version,
/// depending on feature flags.
#[macro_export]
macro_rules! compat_gas {
    ($amount: expr) => {{
        #[cfg(feature = "near-sdk-4")]
        {
            near_sdk::Gas::from($amount)
        }
        #[cfg(feature = "near-sdk-5")]
        {
            near_sdk::Gas::from_gas($amount)
        }
    }};
}

/// **COMPATIBLE (UNSTABLE)**
///
/// Converts a u64 to a gas value for the current `near_sdk` version,
/// depending on feature flags.
#[macro_export]
macro_rules! compat_gas_to_u64 {
    ($amount: expr) => {{
        #[cfg(feature = "near-sdk-4")]
        {
            <near_sdk::Gas as Into<u64>>::into($amount)
        }
        #[cfg(feature = "near-sdk-5")]
        {
            near_sdk::Gas::as_gas($amount)
        }
    }};
}

/// **COMPATIBLE (UNSTABLE)**
///
/// Serializes an expression using Borsh, using the correct version from
/// `near_sdk`, depending on feature flags.
#[macro_export]
macro_rules! compat_borsh_serialize {
    ($e: expr) => {{
        #[cfg(feature = "near-sdk-4")]
        {
            near_sdk::borsh::BorshSerialize::try_to_vec($e)
        }
        #[cfg(feature = "near-sdk-5")]
        {
            near_sdk::borsh::to_vec($e)
        }
    }};
}

/// **COMPATIBLE (UNSTABLE)**
///
/// Imports Borsh from `near_sdk`. Imports the correct items from `near_sdk`
/// versions 4 and 5 depending on feature flags.
#[macro_export]
macro_rules! compat_use_borsh {
    () => {
        $crate::compat_use_borsh!{BorshSerialize, BorshDeserialize}
    };
    ($($i: ident),*) => {
        #[cfg(feature = "near-sdk-4")]
        use near_sdk::borsh;
        use near_sdk::borsh::{$($i),*};
    };
}

/// **COMPATIBLE (UNSTABLE)**
///
/// Implements serde & Borsh serialization and deserialization from `near_sdk`
/// on the given items. Supports `near_sdk` versions 4 and 5 depending on
/// feature flags.
#[macro_export]
macro_rules! compat_derive_serde_borsh {
    (feature = $v: expr, [$($i: ident),*], $($b: tt)+) => {
        #[derive($($i),*)]
        #[cfg_attr(feature = $v, borsh(crate = "near_sdk::borsh"))]
        #[serde(crate = "near_sdk::serde")]
        $($b)+
    };
    ([$($i: ident),*], $($b: tt)+) => {
        $crate::compat_derive_serde_borsh!{feature = "near-sdk-5", [$($i),*], $($b)+}
    };
    ($($b: tt)+) => {
        $crate::compat_derive_serde_borsh!{[Serialize, Deserialize, BorshSerialize, BorshDeserialize], $($b)+}
    };
}

/// **COMPATIBLE (UNSTABLE)**
///
/// Implements serde serialization and deserialization from `near_sdk` on the
/// given items. Supports `near_sdk` versions 4 and 5 depending on feature
/// flags.
#[macro_export]
macro_rules! compat_derive_serde {
    ([$($i: ident),*], $($b: tt)+) => {
        #[derive($($i),*)]
        #[serde(crate = "near_sdk::serde")]
        $($b)+
    };
    ($($b: tt)+) => {
        $crate::compat_derive_serde!{[Serialize, Deserialize], $($b)+}
    };
}

/// **COMPATIBLE (UNSTABLE)**
///
/// Implements Borsh serialization and deserialization from `near_sdk` on the
/// given items. Supports `near_sdk` versions 4 and 5 depending on feature
/// flags.
#[macro_export]
macro_rules! compat_derive_borsh {
    (feature = $v: expr, [$($i: ident),*], $($b: tt)+) => {
        #[derive($($i),*)]
        #[cfg_attr(feature = $v, borsh(crate = "near_sdk::borsh"))]
        $($b)+
    };
    ([$($i: ident),*], $($b: tt)+) => {
        $crate::compat_derive_borsh!{feature = "near-sdk-5", [$($i),*], $($b)+}
    };
    ($($b: tt)+) => {
        $crate::compat_derive_borsh!{[BorshSerialize, BorshDeserialize], $($b)+}
    };
}

/// **COMPATIBLE (UNSTABLE)**
///
/// Implements Borsh serialization and [`near_sdk::BorshStorageKey`] on the
/// given items. Supports `near_sdk` versions 4 and 5 depending on feature
/// flags.
#[macro_export]
macro_rules! compat_derive_storage_key {
    (feature = $v: expr, $($b: tt)+) => {
        #[derive(BorshSerialize, BorshStorageKey)]
        #[cfg_attr(feature = $v, borsh(crate = "near_sdk::borsh"))]
        $($b)+
    };
    ($($b: tt)+) => {
        $crate::compat_derive_storage_key!{feature = "near-sdk-5", $($b)+}
    };
}

use near_sdk::IntoStorageKey;
pub use near_sdk_contract_tools_macros::*;
use once_cell::sync::Lazy;

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
                self, action::*, ext_nep171, ext_nep171_receiver, ext_nep171_resolver, Nep171,
                Nep171Controller, Nep171ControllerInternal, Nep171Receiver, Nep171Resolver, Token,
                TokenId,
            },
            nep177::{
                self, ext_nep177, ContractMetadata, Nep177, Nep177Controller,
                Nep177ControllerInternal, TokenMetadata,
            },
            nep178::{
                self, action::*, ext_nep178, ext_nep178_receiver, ApprovalId, Nep178,
                Nep178Controller, Nep178ControllerInternal, Nep178Receiver, TokenApprovals,
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
