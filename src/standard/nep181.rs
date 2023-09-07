//! NEP-181 non-fungible token contract metadata implementation.
//!
//! Reference: <https://github.com/near/NEPs/blob/master/neps/nep-0181.md>
use std::error::Error;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U64,
    serde::*,
    AccountId, BorshStorageKey,
};
use thiserror::Error;

use crate::{
    slot::Slot,
    standard::{
        nep171::{
            self,
            error::TokenDoesNotExistError,
            event::{NftContractMetadataUpdateLog, NftMetadataUpdateLog},
            *,
        },
        nep297::Event,
    },
    DefaultStorageKey,
};

pub use ext::*;

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    X,
}

/// Internal functions for [`Nep181Controller`].
pub trait Nep181ControllerInternal {
    /// Storage root.
    fn root() -> Slot<()> {
        Slot::root(DefaultStorageKey::Nep181)
    }
}

/// Functions for managing non-fungible tokens with attached metadata, NEP-181.
pub trait Nep181Controller {}

impl<T: Nep181ControllerInternal + Nep171Controller> Nep181Controller for T {}

// separate module with re-export because ext_contract doesn't play well with #![warn(missing_docs)]
mod ext {
    #![allow(missing_docs)]

    use near_sdk::json_types::U128;

    use super::*;

    #[near_sdk::ext_contract(ext_nep181)]
    pub trait Nep181 {
        fn nft_total_supply(&self) -> U128;
        fn nft_tokens(&self, from_index: Option<U128>, limit: Option<u32>) -> Vec<Token>;
        fn nft_supply_for_owner(&self, account_id: AccountId) -> U128;
        fn nft_tokens_for_owner(
            &self,
            account_id: AccountId,
            from_index: Option<U128>,
            limit: Option<u32>,
        ) -> Vec<Token>;
    }
}
