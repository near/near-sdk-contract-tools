//! External interface for NEP-145.
#![allow(missing_docs)] // ext_contract doesn't play nice with #![warn(missing_docs)]

use super::{StorageBalance, StorageBalanceBounds};
use near_sdk::{ext_contract, json_types::U128, AccountId};

/// NEAR uses storage staking which means that a contract account must have
/// sufficient balance to cover all storage added over time. This standard
/// provides a uniform way to pass storage costs onto users.
///
/// # Motivation
///
/// It allows accounts and contracts to:
///
/// - Check an account's storage balance.
/// - Determine the minimum storage needed to add account information such
///     that the account can interact as expected with a contract.
/// - Add storage balance for an account; either one's own or another.
/// - Withdraw some storage deposit by removing associated account data from
///     the contract and then making a call to remove unused deposit.
/// - Unregister an account to recover full storage balance.
#[ext_contract(ext_nep145)]
pub trait Nep145 {
    /// Payable method that receives an attached deposit of NEAR for a given account.
    ///
    /// Returns the updated storage balance record for the given account.
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance;

    /// Withdraw specified amount of available NEAR for predecessor account.
    /// This method is safe to call, and does not remove data.
    ///
    /// Returns the updated storage balance record for the given account.
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance;

    /// Unregister the predecessor account and withdraw all available NEAR.
    ///
    /// Returns `true` iff the account was successfully unregistered.
    /// Returns `false` iff account was not registered before.
    fn storage_unregister(&mut self, force: Option<bool>) -> bool;

    /// Returns the storage balance for the given account, or `None` if the account
    /// is not registered.
    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance>;

    /// Returns minimum and maximum allowed balance amounts to interact with this
    /// contract. See [`StorageBalanceBounds`] for more details.
    fn storage_balance_bounds(&self) -> StorageBalanceBounds;
}
