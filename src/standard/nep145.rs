//! NEP-145 Storage Management
//! <https://github.com/near/NEPs/blob/master/neps/nep-0145.md>

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, ext_contract,
    json_types::U128,
    serde::{Deserialize, Serialize},
    AccountId, BorshStorageKey, Promise,
};
use thiserror::Error;

use crate::{slot::Slot, DefaultStorageKey};

pub use ext::*;

const PANIC_MESSAGE_STORAGE_TOTAL_OVERFLOW: &str = "storage total balance overflow";
const PANIC_MESSAGE_STORAGE_AVAILABLE_OVERFLOW: &str = "storage available balance overflow";
const PANIC_MESSAGE_INCONSISTENT_STATE_AVAILABLE: &str =
    "inconsistent state: available storage balance greater than total storage balance";

/// An account's storage balance.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageBalance {
    total: U128,
    available: U128,
}

impl Default for StorageBalance {
    fn default() -> Self {
        Self {
            total: U128(0),
            available: U128(0),
        }
    }
}

/// Storage balance bounds.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageBalanceBounds {
    min: U128,
    max: Option<U128>,
}

impl Default for StorageBalanceBounds {
    fn default() -> Self {
        Self {
            min: U128(0),
            max: None,
        }
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey<'a> {
    BalanceBounds,
    Account(&'a AccountId),
}

/// NEP-145 Storage Management internal controller interface.
pub trait Nep145ControllerInternal {
    /// NEP-145 lifecycle hook.
    type Hook: Nep145Hook<Self>
    where
        Self: Sized;

    /// Root storage slot.
    fn root() -> Slot<()> {
        Slot::new(DefaultStorageKey::Nep145)
    }

    /// Storage slot for balance bounds.
    fn slot_balance_bounds() -> Slot<StorageBalanceBounds> {
        Slot::new(StorageKey::BalanceBounds)
    }

    /// Storage slot for individual account balance.
    fn slot_account(account_id: &AccountId) -> Slot<StorageBalance> {
        Slot::new(StorageKey::Account(account_id))
    }
}

/// Occurs when an account has insufficient storage balance to perform an operation.
#[derive(Debug, Error)]
#[error(
    "Account {account_id} has insufficient balance: {} available, but attempted to lock {}", available.0, attempted_to_lock.0
)]
pub struct InsufficientBalanceError {
    account_id: AccountId,
    available: U128,
    attempted_to_lock: U128,
}

/// Occurs when an account is not registered.
#[derive(Debug, Error)]
#[error("Account {0} is not registered")]
pub struct AccountNotRegisteredError(AccountId);

/// Occurs when an account attempts to unlock more tokens than it has deposited.
#[derive(Debug, Error)]
#[error("Account {0} cannot unlock more tokens than it has deposited")]
pub struct ExcessiveUnlockError(AccountId);

/// Occurs when an account attempts to withdraw more tokens than the contract
/// allows without unregistering.
#[derive(Debug, Error)]
#[error("Account {account_id} must cover the minimum balance {}", minimum_balance.0)]
pub struct MinimumBalanceUnderrunError {
    account_id: AccountId,
    minimum_balance: U128,
}

/// Occurs when an account attempts to deposit more tokens than the contract
/// allows.
#[derive(Debug, Error)]
#[error("Account {account_id} must not exceed the maximum balance {}", maximum_balance.0)]
pub struct MaximumBalanceOverrunError {
    account_id: AccountId,
    maximum_balance: U128,
}

/// Occurs when an account attempts to unregister with a locked balance.
#[derive(Debug, Error)]
#[error("Account {account_id} cannot unregister with locked balance {} > 0", locked_balance.0)]
pub struct UnregisterWithLockedBalanceError {
    account_id: AccountId,
    locked_balance: U128,
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

/// Errors that can occur when force unregistering storage balance.
#[derive(Debug, Error)]
pub enum StorageForceUnregisterError {
    /// The account is not registered.
    #[error(transparent)]
    AccountNotRegistered(#[from] AccountNotRegisteredError),
}

/// NEP-145 Storage Management controller interface. These functions are not directly
/// exposed to the blockchain.
pub trait Nep145Controller {
    /// NEP-145 lifecycle hook.
    type Hook: Nep145Hook<Self>
    where
        Self: Sized;

    /// Returns the storage balance of the given account.
    fn storage_balance(&self, account_id: &AccountId) -> Option<StorageBalance>;

    /// Locks the given amount of storage balance for the given account.
    fn storage_lock(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageLockError>;

    /// Unlocks the given amount of storage balance for the given account.
    fn storage_unlock(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageUnlockError>;

    /// Deposits the given amount of storage balance for the given account.
    fn storage_deposit(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageDepositError>;

    /// Withdraws the given amount of storage balance for the given account.
    fn storage_withdraw(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageWithdrawError>;

    /// Unregisters the given account, returning the amount of storage balance
    /// that should be refunded.
    fn storage_unregister(
        &mut self,
        account_id: &AccountId,
    ) -> Result<U128, StorageUnregisterError>;

    /// Force unregisters the given account, returning the amount of storage balance
    /// that should be refunded.
    fn storage_force_unregister(
        &mut self,
        account_id: &AccountId,
    ) -> Result<U128, StorageForceUnregisterError>;

    /// Returns the storage balance bounds for the contract.
    fn storage_balance_bounds(&self) -> StorageBalanceBounds;
}

impl<T: Nep145ControllerInternal> Nep145Controller for T {
    type Hook = <Self as Nep145ControllerInternal>::Hook;

    fn storage_balance(&self, account_id: &AccountId) -> Option<StorageBalance> {
        Self::slot_account(account_id).read()
    }

    fn storage_lock(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageLockError> {
        let mut account_slot = Self::slot_account(account_id);
        let mut balance = account_slot
            .read()
            .ok_or(AccountNotRegisteredError(account_id.clone()))?;

        balance.available = balance
            .available
            .0
            .checked_sub(amount.0)
            .ok_or(InsufficientBalanceError {
                account_id: account_id.clone(),
                attempted_to_lock: amount,
                available: balance.available,
            })?
            .into();

        account_slot.write(&balance);

        Ok(balance)
    }

    fn storage_unlock(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageUnlockError> {
        let mut account_slot = Self::slot_account(account_id);

        let mut balance = account_slot
            .read()
            .ok_or(AccountNotRegisteredError(account_id.clone()))?;

        balance.available = {
            let new_available = balance
                .available
                .0
                .checked_add(amount.0)
                .unwrap_or_else(|| env::panic_str(PANIC_MESSAGE_STORAGE_AVAILABLE_OVERFLOW))
                .into();

            if new_available > balance.total {
                return Err(ExcessiveUnlockError(account_id.clone()).into());
            }

            new_available
        };

        account_slot.write(&balance);

        Ok(balance)
    }

    fn storage_deposit(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageDepositError> {
        let mut account_slot = Self::slot_account(account_id);

        let mut balance = account_slot.read().unwrap_or_default();

        balance.total.0 = {
            let new_total = balance
                .total
                .0
                .checked_add(amount.0)
                .unwrap_or_else(|| env::panic_str(PANIC_MESSAGE_STORAGE_TOTAL_OVERFLOW));

            let bounds = self.storage_balance_bounds();

            if new_total < bounds.min.0 {
                return Err(MinimumBalanceUnderrunError {
                    account_id: account_id.clone(),
                    minimum_balance: bounds.min,
                }
                .into());
            }

            if let Some(maximum_balance) = bounds.max {
                if new_total > maximum_balance.0 {
                    return Err(MaximumBalanceOverrunError {
                        account_id: account_id.clone(),
                        maximum_balance,
                    }
                    .into());
                }
            }

            new_total
        };

        account_slot.write(&balance);

        Ok(balance)
    }

    fn storage_withdraw(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageWithdrawError> {
        let mut account_slot = Self::slot_account(account_id);

        let mut balance = account_slot
            .read()
            .ok_or_else(|| AccountNotRegisteredError(account_id.clone()))?;

        balance.total.0 = {
            let bounds = self.storage_balance_bounds();

            balance
                .total
                .0
                .checked_sub(amount.0)
                .filter(|&new_total| new_total >= bounds.min.0)
                .ok_or(MinimumBalanceUnderrunError {
                    account_id: account_id.clone(),
                    minimum_balance: bounds.min,
                })?
        };

        account_slot.write(&balance);

        Ok(balance)
    }

    fn storage_unregister(
        &mut self,
        account_id: &AccountId,
    ) -> Result<U128, StorageUnregisterError> {
        let mut account_slot = Self::slot_account(account_id);

        let balance = account_slot
            .read()
            .ok_or_else(|| AccountNotRegisteredError(account_id.clone()))?;

        match balance.total.0.checked_sub(balance.available.0) {
            Some(locked_balance) if locked_balance > 0 => {
                return Err(UnregisterWithLockedBalanceError {
                    account_id: account_id.clone(),
                    locked_balance: U128(locked_balance),
                }
                .into())
            }
            None => env::panic_str(PANIC_MESSAGE_INCONSISTENT_STATE_AVAILABLE),
            _ => {}
        }

        account_slot.remove();

        Ok(balance.total)
    }

    fn storage_force_unregister(
        &mut self,
        account_id: &AccountId,
    ) -> Result<U128, StorageForceUnregisterError> {
        let mut account_slot = Self::slot_account(account_id);

        let balance = account_slot
            .take()
            .ok_or_else(|| AccountNotRegisteredError(account_id.clone()))?;

        Self::Hook::after_force_unregister(self, account_id, &balance);

        Ok(balance.available)
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        Self::slot_balance_bounds().read().unwrap_or_default()
    }
}

/// NEP-145 lifecycle hook.
pub trait Nep145Hook<C = Self> {
    /// Called after an account force-unregisters. Can be used to clear any
    /// state associated with the account.
    fn after_force_unregister(contract: &mut C, account_id: &AccountId, balance: &StorageBalance);
}

impl<C> Nep145Hook<C> for () {
    fn after_force_unregister(
        _contract: &mut C,
        _account_id: &AccountId,
        _balance: &StorageBalance,
    ) {
    }
}

impl<C, T, U> Nep145Hook<C> for (T, U)
where
    T: Nep145Hook<C>,
    U: Nep145Hook<C>,
{
    fn after_force_unregister(contract: &mut C, account_id: &AccountId, balance: &StorageBalance) {
        T::after_force_unregister(contract, account_id, balance);
        U::after_force_unregister(contract, account_id, balance);
    }
}

// #[near_sdk::near_bindgen]
struct Contract {}

impl Nep145ControllerInternal for Contract {
    type Hook = Self;
}

impl Nep145Hook for Contract {
    fn after_force_unregister(
        contract: &mut Self,
        account_id: &AccountId,
        balance: &StorageBalance,
    ) {
        near_sdk::log!("Force unregister");
    }
}

// #[near_sdk::near_bindgen]
impl Nep145 for Contract {
    // #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        let bounds = Nep145Controller::storage_balance_bounds(self);

        let attached = env::attached_deposit();
        let amount = if registration_only.unwrap_or(false) {
            bounds.min.0
        } else if let Some(U128(max)) = bounds.max {
            u128::min(max, attached)
        } else {
            attached
        };
        let refund = attached.checked_sub(amount).unwrap_or_else(|| {
            env::panic_str(&format!(
                "Attached deposit {} is less than required {}",
                attached, amount
            ))
        });
        let predecessor = env::predecessor_account_id();

        let storage_balance = Nep145Controller::storage_deposit(
            self,
            &account_id.unwrap_or_else(|| predecessor.clone()),
            U128(amount),
        )
        .unwrap_or_else(|e| env::panic_str(&format!("Storage deposit error: {e}")));

        if refund > 0 {
            Promise::new(predecessor).transfer(amount);
        }

        storage_balance
    }

    // #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {
        near_sdk::assert_one_yocto();

        let predecessor = env::predecessor_account_id();

        let balance = Nep145Controller::storage_balance(self, &predecessor)
            .unwrap_or_else(|| env::panic_str("Account is not registered"));

        let amount = amount.unwrap_or(balance.available);

        if amount.0 == 0 {
            return balance;
        }

        let new_balance = Nep145Controller::storage_withdraw(self, &predecessor, amount)
            .unwrap_or_else(|e| env::panic_str(&format!("Storage withdraw error: {e}")));

        Promise::new(predecessor).transfer(amount.0);

        new_balance
    }

    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        near_sdk::assert_one_yocto();

        let predecessor = env::predecessor_account_id();

        let refund = if force.unwrap_or(false) {
            match Nep145Controller::storage_force_unregister(self, &predecessor) {
                Ok(refund) => refund,
                Err(StorageForceUnregisterError::AccountNotRegistered(_)) => return false,
            }
        } else {
            match Nep145Controller::storage_unregister(self, &predecessor) {
                Ok(refund) => refund,
                Err(StorageUnregisterError::UnregisterWithLockedBalance(e)) => {
                    env::panic_str(&format!(
                        "Attempt to unregister from storage with locked balance: {e}"
                    ));
                }
                Err(StorageUnregisterError::AccountNotRegistered(_)) => return false,
            }
        };

        Promise::new(predecessor).transfer(refund.0);
        true
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        Nep145Controller::storage_balance(self, &account_id)
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        Nep145Controller::storage_balance_bounds(self)
    }
}

mod ext {
    #![allow(missing_docs)] // ext_contract doesn't play nice with #![warn(missing_docs)]

    use super::{StorageBalance, StorageBalanceBounds};
    use near_sdk::{ext_contract, json_types::U128, AccountId};

    #[ext_contract(ext_nep145)]
    pub trait Nep145 {
        fn storage_deposit(
            &mut self,
            account_id: Option<AccountId>,
            registration_only: Option<bool>,
        ) -> StorageBalance;

        fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance;

        fn storage_unregister(&mut self, force: Option<bool>) -> bool;

        fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance>;

        fn storage_balance_bounds(&self) -> StorageBalanceBounds;
    }
}
