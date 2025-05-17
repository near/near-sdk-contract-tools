//! Error types for NEP-245 implementations.

use near_sdk::AccountId;
use thiserror::Error;

use super::TokenId;

#[derive(Debug, Error)]
#[error("Token ID {token_id} already exists.")]
pub struct TokenIdCollisionError {
    pub token_id: TokenId,
}

/// Errors that may occur when withdrawing (burning) tokens.
#[derive(Debug, Error)]
pub enum WithdrawError {
    /// The account does not have enough balance to withdraw the given amount.
    #[error(transparent)]
    BalanceUnderflow(#[from] BalanceUnderflowError),
    /// The total supply is less than the amount to be burned.
    #[error(transparent)]
    SupplyUnderflow(#[from] SupplyUnderflowError),
}

/// An account does not have enough balance to withdraw the given amount.
#[derive(Debug, Error)]
#[error("The account {account_id} does not have enough balance of {token_id} to withdraw {amount} (current balance: {balance}).")]
pub struct BalanceUnderflowError {
    /// The token ID.
    pub token_id: TokenId,
    /// The account ID.
    pub account_id: AccountId,
    /// The current balance of the account.
    pub balance: u128,
    /// The amount of the failed withdrawal attempt.
    pub amount: u128,
}

/// The total supply is less than the amount to be burned.
#[derive(Debug, Error)]
#[error("The supply of {token_id} ({supply}) is less than the amount to be burned ({amount}).")]
pub struct SupplyUnderflowError {
    /// The token ID.
    pub token_id: TokenId,
    /// The total supply.
    pub supply: u128,
    /// The amount of the failed withdrawal attempt.
    pub amount: u128,
}

/// Errors that may occur when depositing (minting) tokens.
#[derive(Debug, Error)]
pub enum DepositError {
    /// The balance of the receiver would overflow u128.
    #[error(transparent)]
    BalanceOverflow(#[from] BalanceOverflowError),
    /// The total supply would overflow u128.
    #[error(transparent)]
    SupplyOverflow(#[from] SupplyOverflowError),
}

/// The balance of the account would overflow u128.
#[derive(Debug, Error)]
#[error("The balance of {account_id} of {token_id} ({balance}) plus {amount} would overflow u128.")]
pub struct BalanceOverflowError {
    /// The token ID.
    pub token_id: TokenId,
    /// The account ID.
    pub account_id: AccountId,
    /// The current balance of the account.
    pub balance: u128,
    /// The amount of the failed deposit attempt.
    pub amount: u128,
}

/// The total supply would overflow u128.
#[derive(Debug, Error)]
#[error("The supply of {token_id} ({supply}) plus {amount} would overflow u128.")]
pub struct SupplyOverflowError {
    /// The token ID.
    pub token_id: TokenId,
    /// The total supply.
    pub supply: u128,
    /// The amount of the failed deposit attempt.
    pub amount: u128,
}

/// Errors that may occur when transferring tokens.
#[derive(Debug, Error)]
pub enum TransferError {
    /// The balance of the receiver would overflow u128.
    #[error("Balance of the receiver would overflow u128: {0}")]
    ReceiverBalanceOverflow(#[from] BalanceOverflowError),
    /// The balance of the sender is insufficient.
    #[error("Balance of the sender is insufficient: {0}")]
    SenderBalanceUnderflow(#[from] BalanceUnderflowError),
}
