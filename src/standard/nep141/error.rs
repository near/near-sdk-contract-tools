//! Error types for NEP-141 implementations.

use near_sdk::AccountId;
use thiserror::Error;

/// Errors that may occur when withdrawing (burning) tokens.
#[derive(Debug, Error)]
pub enum WithdrawError {
    /// The account does not have enough balance to withdraw the given amount.
    #[error(transparent)]
    BalanceUnderflow(#[from] BalanceUnderflowError),
    /// The total supply is less than the amount to be burned.
    #[error(transparent)]
    TotalSupplyUnderflow(#[from] TotalSupplyUnderflowError),
}

/// An account does not have enough balance to withdraw the given amount.
#[derive(Debug, Error)]
#[error("The account {account_id} does not have enough balance to withdraw {amount} (current balance: {balance}).")]
pub struct BalanceUnderflowError {
    /// The account ID.
    pub account_id: AccountId,
    /// The current balance of the account.
    pub balance: u128,
    /// The amount of the failed withdrawal attempt.
    pub amount: u128,
}

/// The total supply is less than the amount to be burned.
#[derive(Debug, Error)]
#[error("The total supply ({total_supply}) is less than the amount to be burned ({amount}).")]
pub struct TotalSupplyUnderflowError {
    /// The total supply.
    pub total_supply: u128,
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
    TotalSupplyOverflow(#[from] TotalSupplyOverflowError),
}

/// The balance of the account would overflow u128.
#[derive(Debug, Error)]
#[error("The balance of {account_id} ({balance}) plus {amount} would overflow u128.")]
pub struct BalanceOverflowError {
    /// The account ID.
    pub account_id: AccountId,
    /// The current balance of the account.
    pub balance: u128,
    /// The amount of the failed deposit attempt.
    pub amount: u128,
}

/// The total supply would overflow u128.
#[derive(Debug, Error)]
#[error("The total supply ({total_supply}) plus {amount} would overflow u128.")]
pub struct TotalSupplyOverflowError {
    /// The total supply.
    pub total_supply: u128,
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
