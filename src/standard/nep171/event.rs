//! Event log metadata & associated structures.

use near_sdk::AccountId;
use serde::Serialize;

/// Tokens minted to a single owner.
#[derive(Serialize, Debug, Clone)]
pub struct NftMintLog {
    /// To whom were the new tokens minted?
    pub owner_id: AccountId,
    /// Which tokens were minted?
    pub token_ids: Vec<String>,
    /// Additional mint information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

/// Tokens are transferred from one account to another.
#[derive(Serialize, Debug, Clone)]
pub struct NftTransferLog {
    /// NEP-178 authorized account ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<AccountId>,
    /// Account ID of the previous owner.
    pub old_owner_id: AccountId,
    /// Account ID of the new owner.
    pub new_owner_id: AccountId,
    /// IDs of the transferred tokens.
    pub token_ids: Vec<String>,
    /// Additional transfer information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

/// Tokens are burned from a single holder.
#[derive(Serialize, Debug, Clone)]
pub struct NftBurnLog {
    /// What is the ID of the account from which the tokens were burned?
    pub owner_id: AccountId,
    /// IDs of the burned tokens.
    pub token_ids: Vec<String>,
    /// NEP-178 authorized account ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<AccountId>,
    /// Additional burn information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

/// Token metadata update.
#[derive(Serialize, Debug, Clone)]
pub struct NftMetadataUpdateLog {
    /// IDs of the updated tokens.
    pub token_ids: Vec<String>,
    /// Additional update information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}

/// Contract metadata update.
#[derive(Serialize, Debug, Clone)]
pub struct NftContractMetadataUpdateLog {
    /// Additional update information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
}