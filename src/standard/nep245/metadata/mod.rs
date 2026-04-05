//! NEP-245 metadata extension module.
//!
//! Reference: <https://github.com/near/NEPs/blob/master/neps/nep-0245/Metadata.md>

mod base;
pub use base::BaseMetadata;
mod contract;
pub use contract::ContractMetadata;
pub mod error;
mod ext;
pub use ext::{nep245_metadata, Nep245Metadata};
mod token;
pub use token::{TokenMetadata, TokenMetadataAll};

use near_sdk::{borsh::BorshSerialize, env, near, BorshStorageKey};

use crate::{
    mt::{Nep245Controller, TokenIdRef},
    slot::Slot,
    DefaultStorageKey,
};

/// ID of base token metadata.
pub type BaseMetadataId = String;
/// Referenced ID of base token metadata.
pub type BaseMetadataIdRef = str;

const CONTRACT_METADATA_NOT_INITIALIZED_ERROR: &str = "Contract metadata not initialized";
const ERR_INCONSISTENT_STATE_BASE_METADATA_MISSING: &str =
    "Inconsistent state: base metadata missing";

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey<'a> {
    ContractMetadata,
    BaseMetadata(&'a BaseMetadataIdRef),
    BaseMetadataUsage(&'a BaseMetadataIdRef),
    TokenMetadata(&'a TokenIdRef),
}

macro_rules! builder_fn {
    ($n:ident : $t:ty; $($tail:tt)*) => {
        #[doc = concat!("Set the `", stringify!($n), "` field.")]
        #[must_use]
        pub fn $n(mut self, $n: impl Into<$t>) -> Self {
            self.$n = Some($n.into());
            self
        }

        $crate::standard::nep245::metadata::builder_fn! { $($tail)* }
    };
    () => {};
}
use builder_fn;

/// Token metadata with a reference to the base token metadata by ID.
#[derive(Debug, PartialEq, Eq, Clone)]
#[near(serializers = [borsh])]
pub struct TokenMetadataStore {
    /// ID of the base token metadata.
    pub base_id: BaseMetadataId,
    /// Token-specific metadata.
    pub token: TokenMetadata,
}

/// Internal functions for [`MetadataController`].
pub trait MetadataControllerInternal {
    /// Storage root.
    #[must_use]
    fn root() -> Slot<()> {
        Slot::root(DefaultStorageKey::Nep245).ns(super::StorageKey::Metadata)
    }

    /// Storage slot for contract metadata.
    #[must_use]
    fn slot_contract_metadata() -> Slot<ContractMetadata> {
        Self::root().field(StorageKey::ContractMetadata)
    }

    /// Storage slot for base metadata.
    #[must_use]
    fn slot_base_metadata(base_metadata_id: &BaseMetadataIdRef) -> Slot<BaseMetadata> {
        Self::root().field(StorageKey::BaseMetadata(base_metadata_id))
    }

    /// Storage slot for base metadata usage.
    #[must_use]
    fn slot_base_metadata_usage(base_metadata_id: &BaseMetadataIdRef) -> Slot<u32> {
        Self::root().field(StorageKey::BaseMetadataUsage(base_metadata_id))
    }

    /// Storage slot for token metadata.
    #[must_use]
    fn slot_token_metadata(token_id: &TokenIdRef) -> Slot<TokenMetadataStore> {
        Self::root().field(StorageKey::TokenMetadata(token_id))
    }
}

/// Functions for managing non-fungible tokens with attached metadata, NEP-177.
pub trait MetadataController {
    /// Sets the contract metadata.
    fn set_contract_metadata(&mut self, contract_metadata: &ContractMetadata);

    /// Creates a base token metadata.
    ///
    /// # Errors
    ///
    /// - If the metadata ID already exists.
    fn create_base_metadata(
        &mut self,
        base_metadata: BaseMetadata,
    ) -> Result<(), error::BaseMetadataIdAlreadyExistsError>;

    /// Removes a base token metadata.
    ///
    /// # Errors
    ///
    /// - If the metadata ID does not exist.
    /// - If there are token metadata that still reference this base metadata.
    fn remove_base_metadata(
        &mut self,
        metadata_id: BaseMetadataId,
    ) -> Result<(), error::RemoveBaseTokenMetadataError>;

    /// Sets metadata for a token.
    ///
    /// # Errors
    ///
    /// - If the token ID does not exist.
    /// - If the base token metadata ID does not exist.
    fn set_token_metadata(
        &mut self,
        token_id: &TokenIdRef,
        base_token_metadata_id: BaseMetadataId,
        token_metadata: TokenMetadata,
    ) -> Result<(), error::UpdateTokenMetadataError>;

    /// Retrieves the contract metadata.
    fn contract_metadata(&self) -> ContractMetadata;

    /// Retrieves base token metadata.
    fn base_metadata(&self, base_metadata_id: &BaseMetadataIdRef) -> Option<BaseMetadata>;

    /// Retrieves the metadata for a token ID.
    fn token_metadata(&self, token_id: &TokenIdRef) -> Option<TokenMetadataStore>;
}

impl<T: MetadataControllerInternal + Nep245Controller> MetadataController for T {
    fn set_contract_metadata(&mut self, contract_metadata: &ContractMetadata) {
        Self::slot_contract_metadata().write(contract_metadata);
    }

    fn create_base_metadata(
        &mut self,
        base_metadata: BaseMetadata,
    ) -> Result<(), error::BaseMetadataIdAlreadyExistsError> {
        let mut slot = Self::slot_base_metadata(&base_metadata.id);
        if slot.exists() {
            return Err(error::BaseMetadataIdAlreadyExistsError {
                base_metadata_id: base_metadata.id,
            });
        }

        slot.write(&base_metadata);
        Self::slot_base_metadata_usage(&base_metadata.id).write(&0);
        Ok(())
    }

    fn remove_base_metadata(
        &mut self,
        base_metadata_id: BaseMetadataId,
    ) -> Result<(), error::RemoveBaseTokenMetadataError> {
        let mut slot = Self::slot_base_metadata(&base_metadata_id);
        let mut usage_slot = Self::slot_base_metadata_usage(&base_metadata_id);
        if !slot.exists() {
            return Err(error::BaseMetadataIdDoesNotExistError { base_metadata_id }.into());
        }
        match usage_slot.read() {
            Some(count) if count > 0 => {
                return Err(error::BaseMetadataInUseError {
                    base_metadata_id,
                    count,
                }
                .into());
            }
            _ => {}
        }

        slot.remove();
        usage_slot.remove();
        Ok(())
    }

    fn set_token_metadata(
        &mut self,
        token_id: &TokenIdRef,
        base_metadata_id: BaseMetadataId,
        token_metadata: TokenMetadata,
    ) -> Result<(), error::UpdateTokenMetadataError> {
        if !self.token_exists(token_id) {
            return Err(super::TokenIdDoesNotExistError {
                token_id: token_id.to_string(),
            }
            .into());
        }

        if !Self::slot_base_metadata(&base_metadata_id).exists() {
            return Err(error::BaseMetadataIdDoesNotExistError {
                base_metadata_id: base_metadata_id.to_string(),
            }
            .into());
        }

        let mut slot = Self::slot_token_metadata(token_id);

        let old_base_metadata_id = slot.read().map(|m| m.base_id);

        slot.set(Some(&TokenMetadataStore {
            base_id: base_metadata_id.clone(),
            token: token_metadata,
        }));

        if old_base_metadata_id.as_ref() != Some(&base_metadata_id) {
            if let Some(old_id) = old_base_metadata_id {
                let mut usage_slot = Self::slot_base_metadata_usage(&old_id);
                let usage = usage_slot.read().unwrap_or_else(|| {
                    env::panic_str(ERR_INCONSISTENT_STATE_BASE_METADATA_MISSING)
                });
                usage_slot.write(&(usage - 1));
            }

            let mut usage_slot = Self::slot_base_metadata_usage(&base_metadata_id);
            let usage = usage_slot
                .read()
                .unwrap_or_else(|| env::panic_str(ERR_INCONSISTENT_STATE_BASE_METADATA_MISSING));
            usage_slot.write(&(usage + 1));
        }

        Ok(())
    }

    fn contract_metadata(&self) -> ContractMetadata {
        Self::slot_contract_metadata()
            .read()
            .unwrap_or_else(|| env::panic_str(CONTRACT_METADATA_NOT_INITIALIZED_ERROR))
    }

    fn base_metadata(&self, base_metadata_id: &BaseMetadataIdRef) -> Option<BaseMetadata> {
        Self::slot_base_metadata(base_metadata_id).read()
    }

    fn token_metadata(&self, token_id: &TokenIdRef) -> Option<TokenMetadataStore> {
        Self::slot_token_metadata(token_id).read()
    }
}
