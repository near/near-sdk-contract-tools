//! NEP-148 fungible token metadata implementation
//! <https://github.com/near/NEPs/blob/master/neps/nep-0148.md>

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::Base64VecU8,
    serde::{Deserialize, Serialize},
    BorshStorageKey,
};

use crate::{slot::Slot, DefaultStorageKey};

pub use ext::*;

/// Version of the NEP-148 metadata spec.
pub const FT_METADATA_SPEC: &str = "ft-1.0.0";
/// Error message for unset metadata.
pub const ERR_METADATA_UNSET: &str = "NEP-148 metadata is not set";

/// NEP-148-compatible metadata struct
#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Eq, PartialEq, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct FungibleTokenMetadata {
    /// Version of the NEP-148 spec
    pub spec: String,
    /// Human-friendly name of the token contract
    pub name: String,
    /// Short, ideally unique string to concisely identify the token contract
    pub symbol: String,
    /// String representation (HTTP URL, data URL, IPFS, Arweave, etc.) of an
    /// icon for this token
    pub icon: Option<String>,
    /// External (off-chain) URL to additional JSON metadata for this token contract
    pub reference: Option<String>,
    /// Hash of the content that should be present in the `reference` field.
    /// For tamper protection.
    pub reference_hash: Option<Base64VecU8>,
    /// Cosmetic. Number of base-10 decimal places to shift the floating point.
    /// 24 is a common value.
    pub decimals: u8,
}

impl FungibleTokenMetadata {
    /// Creates a new metadata struct.
    pub fn new(name: String, symbol: String, decimals: u8) -> Self {
        Self {
            spec: FT_METADATA_SPEC.into(),
            name,
            symbol,
            icon: None,
            reference: None,
            reference_hash: None,
            decimals,
        }
    }

    /// Sets the spec field.
    pub fn spec(mut self, spec: String) -> Self {
        self.spec = spec;
        self
    }

    /// Sets the name field.
    pub fn name(mut self, name: String) -> Self {
        self.name = name;
        self
    }

    /// Sets the symbol field.
    pub fn symbol(mut self, symbol: String) -> Self {
        self.symbol = symbol;
        self
    }

    /// Sets the icon field.
    pub fn icon(mut self, icon: String) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Sets the reference field.
    pub fn reference(mut self, reference: String) -> Self {
        self.reference = Some(reference);
        self
    }

    /// Sets the reference_hash field.
    pub fn reference_hash(mut self, reference_hash: Base64VecU8) -> Self {
        self.reference_hash = Some(reference_hash);
        self
    }

    /// Sets the decimals field.
    pub fn decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    Metadata,
}

/// Internal functions for [`Nep148Controller`].
pub trait Nep148ControllerInternal {
    /// Returns the root storage slot for NEP-148.
    fn root() -> Slot<()> {
        Slot::new(DefaultStorageKey::Nep148)
    }

    /// Returns the storage slot for NEP-148 metadata.
    fn metadata() -> Slot<FungibleTokenMetadata> {
        Self::root().field(StorageKey::Metadata)
    }
}

/// Management functions for NEP-148.
pub trait Nep148Controller {
    /// Returns the metadata struct for this contract.
    ///
    /// # Panics
    ///
    /// Panics if the metadata has not been set.
    fn get_metadata(&self) -> FungibleTokenMetadata;

    /// Sets the metadata struct for this contract.
    fn set_metadata(&mut self, metadata: &FungibleTokenMetadata);
}

impl<T: Nep148ControllerInternal> Nep148Controller for T {
    fn get_metadata(&self) -> FungibleTokenMetadata {
        Self::metadata()
            .read()
            .unwrap_or_else(|| env::panic_str(ERR_METADATA_UNSET))
    }

    fn set_metadata(&mut self, metadata: &FungibleTokenMetadata) {
        Self::metadata().set(Some(metadata));
    }
}

mod ext {
    #![allow(missing_docs)] // ext_contract doesn't play well

    use near_sdk::ext_contract;

    use super::FungibleTokenMetadata;

    /// Contract that supports the NEP-148 metadata standard
    #[ext_contract(ext_nep148)]
    pub trait Nep148 {
        /// Returns the metadata struct for this contract.
        fn ft_metadata(&self) -> FungibleTokenMetadata;
    }
}
