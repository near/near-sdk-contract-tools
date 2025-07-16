use near_sdk::{json_types::U64, near};

mod ext;
pub use ext::*;

pub type MetadataId = String;
pub type MetadataIdRef = str;

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey<'a> {
    ContractMetadata,
    BaseMetadata(&'a MetadataIdRef),
    TokenMetadata(&'a TokenIdRef),
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[near(serializers = [borsh, json])]
pub struct ContractMetadata {
    pub spec: String,
    pub name: String,
}

impl ContractMetadata {
    pub const SPEC: &str = "mt-1.0.0";

    pub fn new(name: impl Into<String>) -> Self {
        Self {
            spec: Self::SPEC.to_string(),
            name: name.into(),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
#[near(serializers = [borsh, json])]
pub struct BaseTokenMetadata {
    /// The name of the token, e.g. "Silver Swords" or "Metaverse 3".
    pub name: String,
    /// Unique identifier of the metadata.
    pub id: MetadataId,
    /// Token symbol, e.g. "MOCHI".
    pub symbol: Option<String>,
    /// Data URL of icon.
    pub icon: Option<String>,
    /// Number of decimals to use when representing quantities of this token (in the case of a fungible-like token).
    pub decimals: Option<U64>,
    /// Centralized gateway known to have reliable access to decentralized storage assets referenced by `reference` or `media` URLs.
    pub base_uri: Option<String>,
    /// URL to a JSON file with more information.
    pub reference: Option<String>,
    /// Base64-encoded SHA-256 hash of JSON from the `reference` field. Required if `reference` is not `None`.
    pub reference_hash: Option<String>,
    /// Number of copies of this set of metadata in existence when the token was minted.
    pub copies: Option<U64>,
}

macro_rules! builder_fn {
    ($n:ident : $t:ty; $($tail:tt)*) => {
        #[doc = concat!("Set the `", stringify!($n), "` field.")]
        #[must_use]
        pub fn $n(mut self, $n: impl Into<$t>) -> Self {
            self.$n = Some($n.into());
            self
        }

        builder_fn! { $($tail)* }
    };
    () => {};
}

impl BaseTokenMetadata {
    pub fn new(name: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: id.into(),
            symbol: None,
            icon: None,
            decimals: None,
            base_uri: None,
            reference: None,
            reference_hash: None,
            copies: None,
        }
    }

    builder_fn! {
        symbol: String;
        icon: String;
        decimals: U64;
        base_uri: String;
        reference: String;
        reference_hash: String;
        copies: U64;
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Default)]
#[near(serializers = [json, borsh])]
pub struct TokenMetadata {
    /// The title of the token, e.g. "Arch Nemesis: Mail Carrier" or "Parcel #5055".
    pub title: Option<String>,
    /// Free-form description of the token.
    pub description: Option<String>,
    /// URL to associated media, preferably to decentralized, content-addressed storage.
    pub media: Option<String>,
    /// Base64-encoded SHA-256 hash of content referenced by the `media` field. Required if `media` is included.
    pub media_hash: Option<String>,
    /// When the token was issued or minted, Unix epoch in milliseconds.
    pub issued_at: Option<U64>,
    /// When the token expires, Unix epoch in milliseconds.
    pub expires_at: Option<U64>,
    /// When the token starts being valid, Unix epoch in milliseconds.
    pub starts_at: Option<U64>,
    /// When the token was last updated, Unix epoch in milliseconds.
    pub updated_at: Option<U64>,
    /// Anything extra the token wants to store on-chain. Can be stringified JSON.
    pub extra: Option<String>,
    /// URL to an off-chain JSON file with more info.
    pub reference: Option<String>,
    /// Base64-encoded SHA-256 hash of JSON from the `reference` field. Required if `reference` is included.
    pub reference_hash: Option<String>,
}

impl TokenMetadata {
    pub fn new() -> Self {
        Self::default()
    }

    builder_fn! {
        title: String;
        description: String;
        media: String;
        media_hash: String;
        issued_at: U64;
        expires_at: U64;
        starts_at: U64;
        updated_at: U64;
        extra: String;
        reference: String;
        reference_hash: String;
    }
}

/// Combined metadata for a token, including base metadata and token-specific metadata.
#[derive(PartialEq, Eq, Clone, Debug)]
#[near(serializers = [borsh, json])]
pub struct TokenMetadataAll {
    /// Base metadata that applies to all tokens of this type.
    pub base: BaseTokenMetadata,
    /// Token-specific metadata.
    pub token: TokenMetadata,
}

impl<C> LoadTokenMetadata<C> for TokenMetadata {
    fn load(
        contract: &C,
        token_id: &TokenIdRef,
        metadata: &mut std::collections::HashMap<String, near_sdk::serde_json::Value>,
    ) -> Result<(), Box<dyn Error>> {
        todo!()
    }
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

    /// Storage slot for token metadata.
    #[must_use]
    fn slot_base_metadata(base_metadata_id: &MetadataIdRef) -> Slot<BaseTokenMetadata> {
        Self::root().field(StorageKey::BaseMetadata(base_metadata_id))
    }

    /// Storage slot for token metadata.
    #[must_use]
    fn slot_token_metadata(token_id: &TokenIdRef) -> Slot<TokenMetadata> {
        Self::root().field(StorageKey::TokenMetadata(token_id))
    }
}

pub mod error {
    use super::MetadataId;

    #[derive(Debug, thiserror::Error)]
    #[error("The specified metadata ID already exists: {metadata_id}")]
    pub struct MetadataIdAlreadyExistsError {
        pub metadata_id: MetadataId,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("The specified metadata ID does not exist: {metadata_id}")]
    pub struct MetadataIdDoesNotExistError {
        pub metadata_id: MetadataId,
    }

    #[derive(Debug, thiserror::Error)]
    #[error("Could not mint token with metadata: {0}")]
    pub enum UpdateTokenMetadataError {
        /// The token ID does not exist.
        TokenIdDoesNotExist(#[from] super::TokenIdDoesNotExistError),
        /// The metadata ID does not exist.
        MetadataIdDoesNotExist(#[from] MetadataIdDoesNotExistError),
    }
}

/// Functions for managing non-fungible tokens with attached metadata, NEP-177.
pub trait MetadataController {
    fn create_base_metadata(
        &mut self,
        base_metadata: BaseTokenMetadata,
    ) -> Result<(), error::MetadataIdAlreadyExistsError>;

    fn base_metadata_exists(&self, base_metadata_id: &MetadataIdRef) -> bool;

    fn set_token_metadata(
        &mut self,
        token_id: &TokenIdRef,
        base_metadata_id: &MetadataIdRef,
        token_metadata: &TokenMetadata,
    ) -> Result<(), error::UpdateTokenMetadataError>;

    /// Sets the metadata for a token ID without checking whether the token
    /// exists, etc.
    fn set_token_metadata_unchecked(
        &mut self,
        token_id: &TokenId,
        base_metadata_id: &MetadataIdRef,
        metadata: Option<&TokenMetadata>,
    );

    /// Sets the contract metadata.
    fn set_contract_metadata(&mut self, metadata: &ContractMetadata);

    /// Returns the contract metadata.
    fn contract_metadata(&self) -> ContractMetadata;

    /// Returns the metadata for a token ID.
    fn token_metadata(&self, token_id: &TokenId) -> Option<TokenMetadataAll>;
}

impl<T: MetadataControllerInternal + Nep245Controller> MetadataController for T {
    fn set_token_metadata(
        &mut self,
        token_id: &TokenIdRef,
        base_metadata_id: &MetadataIdRef,
        metadata: &TokenMetadata,
    ) -> Result<(), error::UpdateTokenMetadataError> {
        if !self.token_exists(token_id) {
            return Err(super::TokenIdDoesNotExistError {
                token_id: token_id.to_string(),
            }
            .into());
        }

        if !self.base_metadata_exists(base_metadata_id) {
            return Err(error::MetadataIdDoesNotExistError {
                metadata_id: base_metadata_id.to_string(),
            }
            .into());
        }

        self.set_token_metadata_unchecked(token_id, Some(metadata));
        Ok(())
    }

    fn base_metadata_exists(&self, base_metadata_id: &MetadataIdRef) -> bool {
        Self::slot_base_metadata(base_metadata_id).exists()
    }

    fn set_contract_metadata(&mut self, metadata: &ContractMetadata) {
        Self::slot_contract_metadata().set(Some(metadata));
        Nep171Event::ContractMetadataUpdate(vec![NftContractMetadataUpdateLog { memo: None }])
            .emit();
    }

    fn set_token_metadata_unchecked(
        &mut self,
        token_id: &TokenId,
        metadata: Option<&TokenMetadata>,
    ) {
        <Self as MetadataControllerInternal>::slot_token_metadata(token_id).set(metadata);
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
