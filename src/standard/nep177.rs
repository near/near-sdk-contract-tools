//! NEP-177 non-fungible token contract metadata implementation.
//!
//! Reference: <https://github.com/near/NEPs/blob/master/neps/nep-0177.md>
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    json_types::U64,
    serde::*,
    AccountId, BorshStorageKey,
};
use thiserror::Error;

use crate::{slot::Slot, standard::nep171::*, DefaultStorageKey};

pub struct Token {
    pub token_id: TokenId,
    pub owner_id: AccountId,
    pub metadata: TokenMetadata,
}

impl Token {
    pub fn load(contract: &impl Nep171Controller, token_id: TokenId) -> Option<Self> {
        todo!()
    }
}

#[derive(
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(crate = "near_sdk::serde")]
pub struct ContractMetadata {
    pub spec: String,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub base_uri: Option<String>,
    pub reference: Option<String>,
    pub reference_hash: Option<String>,
}

#[derive(
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub media: Option<String>,
    pub media_hash: Option<String>,
    pub copies: Option<U64>,
    pub issued_at: Option<U64>,
    pub expires_at: Option<U64>,
    pub starts_at: Option<U64>,
    pub updated_at: Option<U64>,
    pub extra: Option<String>,
    pub reference: Option<String>,
    pub reference_hash: Option<String>,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey<'a> {
    ContractMetadata,
    TokenMetadata(&'a TokenId),
}

pub trait Nep177ControllerInternal {
    fn root() -> Slot<()> {
        Slot::root(DefaultStorageKey::Nep177)
    }

    fn slot_contract_metadata() -> Slot<ContractMetadata> {
        Self::root().field(StorageKey::ContractMetadata)
    }

    fn slot_token_metadata(token_id: &TokenId) -> Slot<TokenMetadata> {
        Self::root().field(StorageKey::TokenMetadata(token_id))
    }
}

pub trait Nep177Controller {
    fn update_token_metadata(
        &mut self,
        token_id: &TokenId,
        metadata: TokenMetadata,
    ) -> Result<(), UpdateTokenMetadataError>;

    fn update_contract_metadata(&mut self, metadata: ContractMetadata);
}

#[derive(Error, Debug)]
pub enum UpdateTokenMetadataError {
    #[error(transparent)]
    TokenNotFound(#[from] TokenDoesNotExistError),
}

impl<T: Nep177ControllerInternal + Nep171Controller> Nep177Controller for T {
    fn update_token_metadata(
        &mut self,
        token_id: &TokenId,
        metadata: TokenMetadata,
    ) -> Result<(), UpdateTokenMetadataError> {
        if self.token_owner(token_id).is_some() {
            Self::slot_token_metadata(token_id).set(Some(&metadata));
            Ok(())
        } else {
            Err(TokenDoesNotExistError {
                token_id: token_id.clone(),
            }
            .into())
        }
    }

    fn update_contract_metadata(&mut self, metadata: ContractMetadata) {
        Self::slot_contract_metadata().set(Some(&metadata));
    }
}

// separate module with re-export because ext_contract doesn't play well with #![warn(missing_docs)]
mod ext {
    #![allow(missing_docs)]

    use super::*;

    #[near_sdk::ext_contract(ext_nep171)]
    pub trait Nep177 {
        fn nft_metadata(&self) -> ContractMetadata;
    }
}

pub use ext::*;

use super::nep171::{self, error::TokenDoesNotExistError};
