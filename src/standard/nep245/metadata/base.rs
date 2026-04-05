use near_sdk::{json_types::U64, near};

use super::BaseMetadataId;

/// Base token metadata is shared across multiple tokens.
#[derive(PartialEq, Eq, Clone, Debug)]
#[near(serializers = [borsh, json])]
pub struct BaseMetadata {
    /// The name of the token, e.g. "Silver Swords" or "Metaverse 3".
    pub name: String,
    /// Unique identifier of the metadata.
    pub id: BaseMetadataId,
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

impl BaseMetadata {
    /// Construct a new instance of `BaseTokenMetadata` with the given name and token ID.
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

    super::builder_fn! {
        symbol: String;
        icon: String;
        decimals: U64;
        base_uri: String;
        reference: String;
        reference_hash: String;
        copies: U64;
    }
}
