//! NEP-178 errors.

use super::{TokenId, MAX_APPROVALS};
use near_sdk::AccountId;
use thiserror::Error;

/// Occurs when an account is not authorized to manage approvals for a token.
#[derive(Error, Debug)]
#[error("Account `{account_id}` is not authorized to manage approvals for token `{token_id}`.")]
pub struct UnauthorizedError {
    /// The token ID.
    pub token_id: TokenId,
    /// The unauthorized account ID.
    pub account_id: AccountId,
}

/// The account is already approved for the token.
#[derive(Error, Debug)]
#[error("Account {account_id} is already approved for token {token_id}.")]
pub struct AccountAlreadyApprovedError {
    /// The token ID.
    pub token_id: TokenId,
    /// The account ID that has already been approved.
    pub account_id: AccountId,
}

/// The token has too many approvals.
#[derive(Error, Debug)]
#[error(
    "Too many approvals for token {token_id}, maximum is {}.",
    MAX_APPROVALS
)]
pub struct TooManyApprovalsError {
    /// The token ID.
    pub token_id: TokenId,
}

/// Errors that can occur when managing non-fungible token approvals.
#[derive(Error, Debug)]
pub enum Nep178ApproveError {
    /// The account is not authorized to create approvals for the token.
    #[error(transparent)]
    Unauthorized(#[from] UnauthorizedError),
    /// The account is already approved for the token.
    #[error(transparent)]
    AccountAlreadyApproved(#[from] AccountAlreadyApprovedError),
    /// The token has too many approvals.
    #[error(transparent)]
    TooManyApprovals(#[from] TooManyApprovalsError),
}

/// The account is not approved for the token.
#[derive(Error, Debug)]
#[error("Account {account_id} is not approved for token {token_id}")]
pub struct AccountNotApprovedError {
    /// The token ID.
    pub token_id: TokenId,
    /// The account ID that is not approved.
    pub account_id: AccountId,
}

/// Errors that can occur when revoking non-fungible token approvals.
#[derive(Error, Debug)]
pub enum Nep178RevokeError {
    /// The account is not authorized to revoke approvals for the token.
    #[error(transparent)]
    Unauthorized(#[from] UnauthorizedError),
    /// The account is not approved for the token.
    #[error(transparent)]
    AccountNotApproved(#[from] AccountNotApprovedError),
}

/// Errors that can occur when revoking all approvals for a non-fungible token.
#[derive(Error, Debug)]
pub enum Nep178RevokeAllError {
    /// The account is not authorized to revoke approvals for the token.
    #[error(transparent)]
    Unauthorized(#[from] UnauthorizedError),
}
