//! NEP-245 metadata errors.

use super::BaseMetadataId;

/// The specified base metadata ID already exists.
#[derive(Debug, thiserror::Error)]
#[error("The specified base metadata ID already exists: {base_metadata_id}")]
pub struct BaseMetadataIdAlreadyExistsError {
    /// Base metadata ID.
    pub base_metadata_id: BaseMetadataId,
}

/// The specified base metadata ID does not exist.
#[derive(Debug, thiserror::Error)]
#[error("The specified base metadata ID does not exist: {base_metadata_id}")]
pub struct BaseMetadataIdDoesNotExistError {
    /// Base metadata ID.
    pub base_metadata_id: BaseMetadataId,
}

/// The specified metadata is in use (and cannot be deleted).
#[derive(Debug, thiserror::Error)]
#[error("The specified base metadata is in use by {count} token metadata: {base_metadata_id}")]
pub struct BaseMetadataInUseError {
    /// Base metadata ID.
    pub base_metadata_id: BaseMetadataId,
    /// How many token metadata are using the base metadata ID?
    pub count: u32,
}

/// Could not remove base token metadata.
#[derive(Debug, thiserror::Error)]
#[error("Could not remove base metadata: {0}")]
pub enum RemoveBaseTokenMetadataError {
    /// Base metadata ID does not exist.
    BaseMetadataIdDoesNotExist(#[from] BaseMetadataIdDoesNotExistError),
    /// Base metadata is in use.
    BaseMetadataInUse(#[from] BaseMetadataInUseError),
}

/// Could not update token metadata.
#[derive(Debug, thiserror::Error)]
#[error("Could not mint token with metadata: {0}")]
pub enum UpdateTokenMetadataError {
    /// The token ID does not exist.
    TokenIdDoesNotExist(#[from] super::super::TokenIdDoesNotExistError),
    /// The base metadata ID does not exist.
    BaseMetadataIdDoesNotExist(#[from] BaseMetadataIdDoesNotExistError),
}
