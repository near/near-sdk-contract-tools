//! Event log metadata & associated structures.

use std::borrow::Cow;

use near_sdk::{
    AccountIdRef,
    serde::{Deserialize, Serialize},
};
use near_sdk_contract_tools_macros::event;

/// NEP-171 standard events.
#[event(
    crate = "crate",
    macros = "near_sdk_contract_tools_macros",
    standard = "nep171",
    version = "1.2.0"
)]
#[derive(Debug, Clone)]
pub enum Nep171Event<'a> {
    /// Emitted when a token is newly minted.
    NftMint(Vec<NftMintLog<'a>>),
    /// Emitted when a token is transferred between two parties.
    NftTransfer(Vec<NftTransferLog<'a>>),
    /// Emitted when a token is burned.
    NftBurn(Vec<NftBurnLog<'a>>),
    /// Emitted when the metadata associated with an NFT contract is updated.
    NftMetadataUpdate(Vec<NftMetadataUpdateLog<'a>>),
    /// Emitted when the metadata associated with an NFT contract is updated.
    ContractMetadataUpdate(Vec<NftContractMetadataUpdateLog<'a>>),
}

/// Tokens minted to a single owner.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct NftMintLog<'a> {
    /// To whom were the new tokens minted?
    pub owner_id: Cow<'a, AccountIdRef>,
    /// Which tokens were minted?
    pub token_ids: Vec<Cow<'a, str>>,
    /// Additional mint information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

/// Tokens are transferred from one account to another.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct NftTransferLog<'a> {
    /// NEP-178 authorized account ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<Cow<'a, AccountIdRef>>,
    /// Account ID of the previous owner.
    pub old_owner_id: Cow<'a, AccountIdRef>,
    /// Account ID of the new owner.
    pub new_owner_id: Cow<'a, AccountIdRef>,
    /// IDs of the transferred tokens.
    pub token_ids: Vec<Cow<'a, str>>,
    /// Additional transfer information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

/// Tokens are burned from a single holder.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct NftBurnLog<'a> {
    /// What is the ID of the account from which the tokens were burned?
    pub owner_id: Cow<'a, AccountIdRef>,
    /// IDs of the burned tokens.
    pub token_ids: Vec<Cow<'a, str>>,
    /// NEP-178 authorized account ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<Cow<'a, AccountIdRef>>,
    /// Additional burn information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

/// Token metadata update.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct NftMetadataUpdateLog<'a> {
    /// IDs of the updated tokens.
    pub token_ids: Vec<Cow<'a, str>>,
    /// Additional update information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

/// Contract metadata update.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct NftContractMetadataUpdateLog<'a> {
    /// Additional update information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}
