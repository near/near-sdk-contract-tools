//! Potential errors produced by various token manipulations.

use near_sdk::AccountId;
use thiserror::Error;

use crate::standard::nep178::ApprovalId;

use super::TokenId;

/// Potential errors encountered when performing a burn operation.
#[derive(Error, Clone, Debug)]
pub enum Nep171BurnError {
    /// The token could not be burned because it does not exist.
    #[error(transparent)]
    TokenDoesNotExist(#[from] TokenDoesNotExistError),
    /// The token could not be burned because it is not owned by the expected owner.
    #[error(transparent)]
    TokenNotOwnedByExpectedOwner(#[from] TokenNotOwnedByExpectedOwnerError),
}

/// Potential errors encountered when attempting to mint a new token.
#[derive(Error, Clone, Debug)]
pub enum Nep171MintError {
    /// The token could not be minted because a token with the same ID already exists.
    #[error(transparent)]
    TokenAlreadyExists(#[from] TokenAlreadyExistsError),
}

/// Potential errors encountered when performing a token transfer.
#[derive(Error, Clone, Debug)]
pub enum Nep171TransferError {
    /// The token could not be transferred because it does not exist.
    #[error(transparent)]
    TokenDoesNotExist(#[from] TokenDoesNotExistError),
    /// The token could not be transferred because the sender is not allowed to perform transfers of this token on behalf of its current owner. See: NEP-178.
    ///
    /// NOTE: If you only implement NEP-171, approval IDs will _not work_, and this error will always be returned whenever the sender is not the current owner.
    #[error(transparent)]
    SenderNotApproved(#[from] SenderNotApprovedError),
    /// The token could not be transferred because the token is being sent to the account that currently owns it. Reflexive transfers are not allowed.
    #[error(transparent)]
    TokenReceiverIsCurrentOwner(#[from] TokenReceiverIsCurrentOwnerError),
    /// The token could not be transferred because it is no longer owned by the expected owner.
    #[error(transparent)]
    TokenNotOwnedByExpectedOwner(#[from] TokenNotOwnedByExpectedOwnerError),
}

/// Occurs when trying to create a token ID that already exists.
/// Overwriting pre-existing token IDs is not allowed.
#[derive(Error, Clone, Debug)]
#[error("Token `{token_id}` already exists")]
pub struct TokenAlreadyExistsError {
    /// The conflicting token ID.
    pub token_id: TokenId,
}

/// When attempting to interact with a non-existent token ID.
#[derive(Error, Clone, Debug)]
#[error("Token `{token_id}` does not exist")]
pub struct TokenDoesNotExistError {
    /// The invalid token ID.
    pub token_id: TokenId,
}

/// Occurs when performing a checked operation that expects a token to be
/// owned by a particular account, but the token is _not_ owned by that
/// account.
#[derive(Error, Clone, Debug)]
#[error("Token `{token_id}` is owned by `{owner_id}` instead of expected `{expected_owner_id}`")]
pub struct TokenNotOwnedByExpectedOwnerError {
    /// The token was supposed to be owned by this account.
    pub expected_owner_id: AccountId,
    /// The token is actually owned by this account.
    pub owner_id: AccountId,
    /// The ID of the token in question.
    pub token_id: TokenId,
}

/// Occurs when a particular account is not allowed to transfer a token (e.g. on behalf of another user). See: NEP-178.
#[derive(Error, Clone, Debug)]
#[error("Sender `{sender_id}` does not have permission to transfer token `{token_id}`, owned by `{owner_id}`, with approval ID {approval_id}")]
pub struct SenderNotApprovedError {
    /// The unapproved sender.
    pub sender_id: AccountId,
    /// The owner of the token.
    pub owner_id: AccountId,
    /// The ID of the token in question.
    pub token_id: TokenId,
    /// The approval ID that the sender tried to use to transfer the token.
    pub approval_id: ApprovalId,
}

/// Occurs when attempting to perform a transfer of a token from one
/// account to the same account.
#[derive(Error, Clone, Debug)]
#[error(
    "Receiver must be different from current owner `{owner_id}` to transfer token `{token_id}`"
)]
pub struct TokenReceiverIsCurrentOwnerError {
    /// The account ID of current owner of the token.
    pub owner_id: AccountId,
    /// The ID of the token in question.
    pub token_id: TokenId,
}
