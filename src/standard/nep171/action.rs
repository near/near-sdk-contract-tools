//! NEP-171 actions.
//!
//! Used when calling various functions on [`Nep171Controller`]. Also used when
//! implementing [`Hook`]s for the NEP-171 component.

use super::*;
use near_sdk::{
    borsh::{self, BorshSerialize},
    serde::Serialize,
};

/// NEP-171 mint action.
#[derive(Clone, Debug, Serialize, BorshSerialize, PartialEq, Eq)]
pub struct Nep171Mint<'a> {
    /// Token IDs to mint.
    pub token_ids: &'a [TokenId],
    /// Account ID of the receiver.
    pub receiver_id: &'a AccountId,
    /// Optional memo string.
    pub memo: Option<&'a str>,
}

/// NEP-171 burn action.
#[derive(Clone, Debug, Serialize, BorshSerialize, PartialEq, Eq)]
pub struct Nep171Burn<'a> {
    /// Token IDs to burn.
    pub token_ids: &'a [TokenId],
    /// Account ID of the owner.
    pub owner_id: &'a AccountId,
    /// Optional memo string.
    pub memo: Option<&'a str>,
}

/// Transfer metadata generic over both types of transfer (`nft_transfer` and
/// `nft_transfer_call`).
#[derive(Serialize, BorshSerialize, PartialEq, Eq, Clone, Debug, Hash)]
pub struct Nep171Transfer<'a> {
    /// Why is this sender allowed to perform this transfer?
    pub authorization: Nep171TransferAuthorization,
    /// Sending account ID.
    pub sender_id: &'a AccountId,
    /// Receiving account ID.
    pub receiver_id: &'a AccountId,
    /// Token ID.
    pub token_id: &'a TokenId,
    /// Optional memo string.
    pub memo: Option<&'a str>,
    /// Message passed to contract located at `receiver_id` in the case of `nft_transfer_call`.
    pub msg: Option<&'a str>,
    /// `true` if the transfer is a revert for a `nft_transfer_call`.
    pub revert: bool,
}
