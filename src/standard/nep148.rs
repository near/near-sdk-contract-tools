//! NEP-148 fungible token metadata implementation
//! <https://github.com/near/NEPs/blob/master/neps/nep-0148.md>

use near_sdk::{BorshStorageKey, env, json_types::Base64VecU8, near};

use crate::{DefaultStorageKey, slot::Slot};

pub use ext::*;

/// Version of the NEP-148 metadata spec.
pub const FT_METADATA_SPEC: &str = "ft-1.0.0";
/// Error message for unset metadata.
pub const ERR_METADATA_UNSET: &str = "NEP-148 metadata is not set";

/// NEP-148-compatible metadata struct
#[derive(Eq, PartialEq, Clone, Debug)]
#[near(serializers = [borsh, json])]
pub struct ContractMetadata {
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

impl ContractMetadata {
    /// Creates a new metadata struct.
    #[must_use]
    pub fn new(name: impl Into<String>, symbol: impl Into<String>, decimals: u8) -> Self {
        Self {
            spec: FT_METADATA_SPEC.into(),
            name: name.into(),
            symbol: symbol.into(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals,
        }
    }

    /// Sets the spec field.
    #[must_use]
    pub fn spec(mut self, spec: impl Into<String>) -> Self {
        self.spec = spec.into();
        self
    }

    /// Sets the name field.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Sets the symbol field.
    #[must_use]
    pub fn symbol(mut self, symbol: impl Into<String>) -> Self {
        self.symbol = symbol.into();
        self
    }

    /// Sets the icon field.
    #[must_use]
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets the reference field.
    #[must_use]
    pub fn reference(mut self, reference: impl Into<String>) -> Self {
        self.reference = Some(reference.into());
        self
    }

    /// Sets the `reference_hash` field.
    #[must_use]
    pub fn reference_hash(mut self, reference_hash: impl Into<Base64VecU8>) -> Self {
        self.reference_hash = Some(reference_hash.into());
        self
    }

    /// Sets the decimals field.
    #[must_use]
    pub fn decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }
}

#[derive(BorshStorageKey)]
#[near]
enum StorageKey {
    Metadata,
}

/// Internal functions for [`Nep148Controller`].
pub trait Nep148ControllerInternal {
    /// Returns the root storage slot for NEP-148.
    #[must_use]
    fn root() -> Slot<()> {
        Slot::new(DefaultStorageKey::Nep148)
    }

    /// Returns the storage slot for NEP-148 metadata.
    #[must_use]
    fn metadata() -> Slot<ContractMetadata> {
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
    fn get_metadata(&self) -> ContractMetadata;

    /// Sets the metadata struct for this contract.
    fn set_metadata(&mut self, metadata: &ContractMetadata);
}

impl<T: Nep148ControllerInternal> Nep148Controller for T {
    fn get_metadata(&self) -> ContractMetadata {
        Self::metadata()
            .read()
            .unwrap_or_else(|| env::panic_str(ERR_METADATA_UNSET))
    }

    fn set_metadata(&mut self, metadata: &ContractMetadata) {
        Self::metadata().set(Some(metadata));
    }
}

mod ext {
    #![allow(missing_docs)] // ext_contract doesn't play well

    use near_sdk::ext_contract;

    use super::ContractMetadata;

    /// Contract that supports the NEP-148 metadata standard
    #[ext_contract(ext_nep148)]
    pub trait Nep148 {
        /// Returns the metadata struct for this contract.
        fn ft_metadata(&self) -> ContractMetadata;
    }
}
