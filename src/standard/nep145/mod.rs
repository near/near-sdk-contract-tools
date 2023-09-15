//! NEP-145 Storage Management
//! <https://github.com/near/NEPs/blob/master/neps/nep-0145.md>

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    serde::{Deserialize, Serialize},
    AccountId, BorshStorageKey,
};

use crate::{slot::Slot, DefaultStorageKey};

pub mod error;
use error::*;

mod ext;
pub use ext::*;

const PANIC_MESSAGE_STORAGE_TOTAL_OVERFLOW: &str = "storage total balance overflow";
const PANIC_MESSAGE_STORAGE_AVAILABLE_OVERFLOW: &str = "storage available balance overflow";
const PANIC_MESSAGE_INCONSISTENT_STATE_AVAILABLE: &str =
    "inconsistent state: available storage balance greater than total storage balance";

/// An account's storage balance.
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub struct StorageBalance {
    /// The total amount of storage balance.
    pub total: U128,

    /// The amount of storage balance that is available for use.
    pub available: U128,
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
    /// The minimum storage balance.
    pub min: U128,

    /// The maximum storage balance.
    pub max: Option<U128>,
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

    /// Sets the storage balance bounds for the contract.
    fn set_storage_balance_bounds(&mut self, bounds: &StorageBalanceBounds);
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

        balance.available.0 += amount.0;

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

    fn set_storage_balance_bounds(&mut self, bounds: &StorageBalanceBounds) {
        Self::slot_balance_bounds().write(bounds);
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
