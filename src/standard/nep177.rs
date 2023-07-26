//! NEP-177 non-fungible token contract metadata implementation.
//!
//! Reference: <https://github.com/near/NEPs/blob/master/neps/nep-0177.md>
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
            error::{TokenAlreadyExistsError, TokenDoesNotExistError},
            event::{NftContractMetadataUpdateLog, NftMetadataUpdateLog},
            *,
        },
        nep297::Event,
    },
    DefaultStorageKey,
};

const CONTRACT_METADATA_NOT_INITIALIZED_ERROR: &str = "Contract metadata not initialized";

#[derive(
    Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize, BorshSerialize, BorshDeserialize,
)]
#[serde(crate = "near_sdk::serde")]
pub struct Token {
    pub token_id: TokenId,
    pub owner_id: AccountId,
    pub metadata: TokenMetadata,
}

impl Token {
    pub fn load(
        contract: &(impl Nep171Controller + Nep177Controller),
        token_id: TokenId,
    ) -> Option<Self> {
        let owner_id = contract.token_owner(&token_id)?;
        let metadata = contract.token_metadata(&token_id)?;
        Some(Self {
            token_id,
            owner_id,
            metadata,
        })
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

impl ContractMetadata {
    pub const SPEC: &'static str = "nft-1.0.0";

    pub fn new(name: String, symbol: String, base_uri: Option<String>) -> Self {
        Self {
            spec: Self::SPEC.to_string(),
            name,
            symbol,
            icon: None,
            base_uri,
            reference: None,
            reference_hash: None,
        }
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
    fn mint_with_metadata(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        metadata: TokenMetadata,
    ) -> Result<(), Nep171MintError>;

    fn burn_with_metadata(
        &mut self,
        token_id: TokenId,
        current_owner_id: &AccountId,
    ) -> Result<(), Nep171BurnError>;

    fn set_token_metadata_unchecked(&mut self, token_id: TokenId, metadata: Option<TokenMetadata>);

    fn set_token_metadata(
        &mut self,
        token_id: TokenId,
        metadata: TokenMetadata,
    ) -> Result<(), UpdateTokenMetadataError>;

    fn set_contract_metadata(&mut self, metadata: ContractMetadata);

    fn contract_metadata(&self) -> ContractMetadata;

    fn token_metadata(&self, token_id: &TokenId) -> Option<TokenMetadata>;
}

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum UpdateTokenMetadataError {
    #[error(transparent)]
    TokenNotFound(#[from] TokenDoesNotExistError),
}

impl<T: Nep177ControllerInternal + Nep171Controller> Nep177Controller for T {
    fn set_token_metadata(
        &mut self,
        token_id: TokenId,
        metadata: TokenMetadata,
    ) -> Result<(), UpdateTokenMetadataError> {
        if self.token_owner(&token_id).is_some() {
            self.set_token_metadata_unchecked(token_id, Some(metadata));
            Ok(())
        } else {
            Err(TokenDoesNotExistError { token_id }.into())
        }
    }

    fn set_contract_metadata(&mut self, metadata: ContractMetadata) {
        Self::slot_contract_metadata().set(Some(&metadata));
        Nep171Event::ContractMetadataUpdate(vec![NftContractMetadataUpdateLog { memo: None }])
            .emit();
    }

    fn mint_with_metadata(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        metadata: TokenMetadata,
    ) -> Result<(), Nep171MintError> {
        let token_ids = [token_id];
        self.mint(&token_ids, &owner_id, None)?;
        let [token_id] = token_ids;
        self.set_token_metadata_unchecked(token_id, Some(metadata));
        Ok(())
    }

    fn burn_with_metadata(
        &mut self,
        token_id: TokenId,
        current_owner_id: &AccountId,
    ) -> Result<(), Nep171BurnError> {
        let token_ids = [token_id];
        self.burn(&token_ids, current_owner_id, None)?;
        let [token_id] = token_ids;
        self.set_token_metadata_unchecked(token_id, None);
        Ok(())
    }

    fn set_token_metadata_unchecked(&mut self, token_id: TokenId, metadata: Option<TokenMetadata>) {
        <Self as Nep177ControllerInternal>::slot_token_metadata(&token_id).set(metadata.as_ref());
        nep171::Nep171Event::NftMetadataUpdate(vec![NftMetadataUpdateLog {
            token_ids: vec![token_id],
            memo: None,
        }])
        .emit();
    }

    fn token_metadata(&self, token_id: &TokenId) -> Option<TokenMetadata> {
        <Self as Nep177ControllerInternal>::slot_token_metadata(token_id).read()
    }

    fn contract_metadata(&self) -> ContractMetadata {
        Self::slot_contract_metadata()
            .read()
            .unwrap_or_else(|| env::panic_str(CONTRACT_METADATA_NOT_INITIALIZED_ERROR))
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
