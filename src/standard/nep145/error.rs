//! Error types for the NEP-145 standard.

use near_sdk::{json_types::U128, AccountId};
use thiserror::Error;

/// Occurs when an account has insufficient storage balance to perform an operation.
#[derive(Debug, Error)]
#[error(
    "Account {account_id} has insufficient balance: {} available, but attempted to lock {}", available.0, attempted_to_lock.0
)]
pub struct InsufficientBalanceError {
    /// The account that attempted to perform the operation.
    pub account_id: AccountId,

    /// The amount of storage balance available to the account.
    pub available: U128,

    /// The amount of storage balance the account attempted to lock.
    pub attempted_to_lock: U128,
}

/// Occurs when an account is not registered.
#[derive(Debug, Error)]
#[error("Account {0} is not registered")]
pub struct AccountNotRegisteredError(pub AccountId);

/// Occurs when an account attempts to unlock more tokens than it has deposited.
#[derive(Debug, Error)]
#[error("Account {0} cannot unlock more tokens than it has deposited")]
pub struct ExcessiveUnlockError(pub AccountId);

/// Occurs when an account attempts to withdraw more tokens than the contract
/// allows without unregistering.
#[derive(Debug, Error)]
#[error("Account {account_id} must cover the minimum balance {}", minimum_balance.0)]
pub struct MinimumBalanceUnderrunError {
    /// The account that attempted to perform the operation.
    pub account_id: AccountId,

    /// The minimum balance required to remain registered.
    pub minimum_balance: U128,
}

/// Occurs when an account attempts to deposit more tokens than the contract
/// allows.
#[derive(Debug, Error)]
#[error("Account {account_id} must not exceed the maximum balance {}", maximum_balance.0)]
pub struct MaximumBalanceOverrunError {
    /// The account that attempted to perform the operation.
    pub account_id: AccountId,

    /// The maximum balance allowed.
    pub maximum_balance: U128,
}

/// Occurs when an account attempts to unregister with a locked balance.
#[derive(Debug, Error)]
#[error("Account {account_id} cannot unregister with locked balance {} > 0", locked_balance.0)]
pub struct UnregisterWithLockedBalanceError {
    /// The account that attempted to perform the operation.
    pub account_id: AccountId,

    /// The amount of storage balance locked by the account.
    pub locked_balance: U128,
}

/// Errors that can occur when locking storage balance.
#[derive(Debug, Error)]
pub enum StorageLockError {
    /// The account is not registered.
    #[error(transparent)]
    AccountNotRegistered(#[from] AccountNotRegisteredError),
    /// The account has insufficient balance.
    #[error(transparent)]
    InsufficientBalance(#[from] InsufficientBalanceError),
}

/// Errors that can occur when unlocking storage balance.
#[derive(Debug, Error)]
pub enum StorageUnlockError {
    /// The account is not registered.
    #[error(transparent)]
    AccountNotRegistered(#[from] AccountNotRegisteredError),
    /// The account tried to unlock more tokens than it has deposited.
    #[error(transparent)]
    ExcessiveUnlock(#[from] ExcessiveUnlockError),
}

/// Errors that can occur when depositing storage balance.
#[derive(Debug, Error)]
pub enum StorageDepositError {
    /// The deposit does not meet the minimum balance requirement.
    #[error(transparent)]
    MinimumBalanceUnderrun(#[from] MinimumBalanceUnderrunError),
    /// The deposit exceeds the maximum balance limit.
    #[error(transparent)]
    MaximumBalanceOverrunError(#[from] MaximumBalanceOverrunError),
}

/// Errors that can occur when withdrawing storage balance.
#[derive(Debug, Error)]
pub enum StorageWithdrawError {
    /// The account is not registered.
    #[error(transparent)]
    AccountNotRegistered(#[from] AccountNotRegisteredError),
    /// The withdrawal does not meet the minimum balance requirement.
    #[error(transparent)]
    MinimumBalanceUnderrun(#[from] MinimumBalanceUnderrunError),
}

/// Errors that can occur when unregistering storage balance.
#[derive(Debug, Error)]
pub enum StorageUnregisterError {
    /// The account is not registered.
    #[error(transparent)]
    AccountNotRegistered(#[from] AccountNotRegisteredError),
    /// The account has a locked balance (is still using storage somewhere),
    /// and cannot be unregistered.
    #[error(transparent)]
    UnregisterWithLockedBalance(#[from] UnregisterWithLockedBalanceError),
}

/// Errors that can occur when force-unregistering storage balance.
#[derive(Debug, Error)]
pub enum StorageForceUnregisterError {
    /// The account is not registered.
    #[error(transparent)]
    AccountNotRegistered(#[from] AccountNotRegisteredError),
}

/// Errors that can occur when performing storage accounting.
#[derive(Debug, Error)]
pub enum StorageAccountingError {
    /// Storage lock error.
    #[error(transparent)]
    StorageLockError(#[from] StorageLockError),

    /// Storage unlock error.
    #[error(transparent)]
    StorageUnlockError(#[from] StorageUnlockError),
}
