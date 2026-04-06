use near_sdk::{json_types::U64, near, serde_json};

use crate::mt::{
    TokenIdRef,
    nep245::{
        LoadTokenMetadata,
        metadata::{BaseMetadata, MetadataController},
    },
};

/// Token-specific metadata.
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
    /// Creates a new empty `TokenMetadata`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    super::builder_fn! {
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
    pub base: BaseMetadata,
    /// Token-specific metadata.
    pub token: TokenMetadata,
}

impl<C: MetadataController> LoadTokenMetadata<C> for TokenMetadata {
    fn load(
        contract: &C,
        token_id: &TokenIdRef,
        metadata: &mut std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(stored) = contract.token_metadata(token_id) else {
            return Ok(());
        };

        metadata.insert(
            "base_metadata_id".to_string(),
            serde_json::to_value(stored.base_id)?,
        );
        metadata.insert(
            "token_metadata".to_string(),
            serde_json::to_value(stored.token)?,
        );
        Ok(())
    }
}
