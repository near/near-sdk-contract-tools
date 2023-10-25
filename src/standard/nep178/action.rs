//! NEP-178 actions.
//!
//! Used when calling various functions on [`Nep178Controller`]. Also used when
//! implementing [`Hook`]s for the NEP-178 component.

use super::*;
use near_sdk::{
    borsh::{self, BorshSerialize},
    serde::Serialize,
};

/// NEP-178 approve action.
#[derive(Clone, Debug, Serialize, BorshSerialize, PartialEq, Eq)]
pub struct Nep178Approve<'a> {
    pub token_id: &'a TokenId,
    pub current_owner_id: &'a AccountId,
    pub account_id: &'a AccountId,
}

/// NEP-178 revoke action.
#[derive(Clone, Debug, Serialize, BorshSerialize, PartialEq, Eq)]
pub struct Nep178Revoke<'a> {
    pub token_id: &'a TokenId,
    pub current_owner_id: &'a AccountId,
    pub account_id: &'a AccountId,
}

/// NEP-178 revoke all action.
#[derive(Clone, Debug, Serialize, BorshSerialize, PartialEq, Eq)]
pub struct Nep178RevokeAll<'a> {
    pub token_id: &'a TokenId,
    pub current_owner_id: &'a AccountId,
}
