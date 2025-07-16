#![allow(missing_docs)]

use near_sdk::ext_contract;

pub use super::super::*;
pub use super::*;

/// NEP-245 Metadata contract interface.
#[ext_contract(nep245_metadata)]
pub trait Nep245Metadata {
    /// Returns the top-level contract level metadata.
    fn mt_metadata_contract(&self) -> ContractMetadata;
    /// Returns the complete metadata for a list of tokens.
    fn mt_metadata_token_all(&self, token_ids: Vec<TokenId>) -> Vec<TokenMetadataAll>;
    /// Returns the token-specific metadata for a list of tokens by their IDs.
    fn mt_metadata_token_by_token_id(&self, token_ids: Vec<TokenId>) -> Vec<TokenMetadata>;
    /// Returns the base metadata for a list of tokens by their IDs.
    fn mt_metadata_base_by_token_id(&self, token_ids: Vec<String>) -> Vec<BaseTokenMetadata>;
    /// Returns a list of base metadata by their IDs.
    fn mt_metadata_base_by_metadata_id(
        &self,
        base_metadata_ids: Vec<String>,
    ) -> Vec<BaseTokenMetadata>;
}
