//! NEP-145 Storage Management
//! <https://github.com/near/NEPs/blob/master/neps/nep-0145.md>

use std::{borrow::Cow, cmp::Ordering};

use near_sdk::{AccountIdRef, BorshStorageKey, NearToken, borsh::BorshSerialize, env, near};

use crate::{DefaultStorageKey, hook::Hook, slot::Slot};

pub mod error;
use error::*;
mod ext;
pub use ext::*;
pub mod hooks;

const PANIC_MESSAGE_STORAGE_TOTAL_OVERFLOW: &str = "storage total balance overflow";
const PANIC_MESSAGE_STORAGE_AVAILABLE_OVERFLOW: &str = "storage available balance overflow";
const PANIC_MESSAGE_STORAGE_FEE_OVERFLOW: &str = "storage fee overflow";
const PANIC_MESSAGE_STORAGE_CREDIT_OVERFLOW: &str = "storage credit overflow";
const PANIC_MESSAGE_INCONSISTENT_STATE_AVAILABLE: &str =
    "inconsistent state: available storage balance greater than total storage balance";

/// An account's storage balance.
#[derive(Clone, Debug, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub struct StorageBalance {
    /// The total amount of storage balance.
    pub total: NearToken,

    /// The amount of storage balance that is available for use.
    pub available: NearToken,
}

impl Default for StorageBalance {
    fn default() -> Self {
        Self {
            total: NearToken::from_yoctonear(0),
            available: NearToken::from_yoctonear(0),
        }
    }
}

/// Storage balance bounds.
#[derive(Clone, Debug, PartialEq, Eq)]
#[near(serializers = [borsh, json])]
pub struct StorageBalanceBounds {
    /// The minimum storage balance.
    pub min: NearToken,

    /// The maximum storage balance.
    pub max: Option<NearToken>,
}

impl StorageBalanceBounds {
    /// Restricts a balance to be within the bounds.
    #[must_use]
    pub fn bound(&self, balance: NearToken, registration_only: bool) -> NearToken {
        if registration_only {
            self.min
        } else if let Some(max) = self.max {
            NearToken::from_yoctonear(u128::min(max.as_yoctonear(), balance.as_yoctonear()))
        } else {
            balance
        }
    }
}

impl Default for StorageBalanceBounds {
    fn default() -> Self {
        Self {
            min: NearToken::from_yoctonear(0),
            max: None,
        }
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey<'a> {
    BalanceBounds,
    Account(&'a AccountIdRef),
}

/// Describes a force unregister action.
#[derive(Clone, Debug, PartialEq, Eq)]
#[near]
pub struct Nep145ForceUnregister<'a> {
    /// The account to be unregistered.
    pub account_id: Cow<'a, AccountIdRef>,
    /// The account's balance at the time of unregistration.
    pub balance: StorageBalance,
}

/// NEP-145 Storage Management internal controller interface.
pub trait Nep145ControllerInternal {
    /// NEP-145 lifecycle hook.
    type ForceUnregisterHook: for<'a> Hook<Self, Nep145ForceUnregister<'a>>
    where
        Self: Sized;

    /// Root storage slot.
    #[must_use]
    fn root() -> Slot<()> {
        Slot::new(DefaultStorageKey::Nep145)
    }

    /// Storage slot for balance bounds.
    #[must_use]
    fn slot_balance_bounds() -> Slot<StorageBalanceBounds> {
        Slot::new(StorageKey::BalanceBounds)
    }

    /// Storage slot for individual account balance.
    #[must_use]
    fn slot_account(account_id: &AccountIdRef) -> Slot<StorageBalance> {
        Slot::new(StorageKey::Account(account_id))
    }
}

/// NEP-145 Storage Management controller interface. These functions are not directly
/// exposed to the blockchain.
pub trait Nep145Controller {
    /// NEP-145 lifecycle hook.
    type ForceUnregisterHook: for<'a> Hook<Self, Nep145ForceUnregister<'a>>
    where
        Self: Sized;

    /// Returns the storage balance of the given account.
    ///
    /// # Errors
    ///
    /// - If the account is not registered.
    fn get_storage_balance(
        &self,
        account_id: &AccountIdRef,
    ) -> Result<StorageBalance, AccountNotRegisteredError>;

    /// Locks the given amount of storage balance for the given account.
    ///
    /// # Errors
    ///
    /// - If the account is not registered.
    /// - If the account's balance is too low.
    fn lock_storage(
        &mut self,
        account_id: &AccountIdRef,
        amount: NearToken,
    ) -> Result<StorageBalance, StorageLockError>;

    /// Unlocks the given amount of storage balance for the given account.
    ///
    /// # Errors
    ///
    /// - If the account is not registered.
    /// - If the account attempts to unlock more tokens than are deposited.
    fn unlock_storage(
        &mut self,
        account_id: &AccountIdRef,
        amount: NearToken,
    ) -> Result<StorageBalance, StorageUnlockError>;

    /// Deposits the given amount of storage balance for the given account.
    ///
    /// # Errors
    ///
    /// - If the account balance would be less than the minimum balance.
    /// - If the account balance would be greater than the maximum balance.
    fn deposit_to_storage_account(
        &mut self,
        account_id: &AccountIdRef,
        amount: NearToken,
    ) -> Result<StorageBalance, StorageDepositError>;

    /// Withdraws the given amount of storage balance for the given account.
    ///
    /// # Errors
    ///
    /// - If the account is not registered.
    /// - If the account attempts to withdraw more tokens than are deposited.
    /// - If the account balance would be less than the minimum balance.
    fn withdraw_from_storage_account(
        &mut self,
        account_id: &AccountIdRef,
        amount: NearToken,
    ) -> Result<StorageBalance, StorageWithdrawError>;

    /// Unregisters the given account, returning the amount of storage balance
    /// that should be refunded.
    ///
    /// # Errors
    ///
    /// - If the account is not registered.
    /// - If the account still has a locked balance.
    fn unregister_storage_account(
        &mut self,
        account_id: &AccountIdRef,
    ) -> Result<NearToken, StorageUnregisterError>;

    /// Force unregisters the given account, returning the amount of storage balance
    /// that should be refunded.
    ///
    /// # Errors
    ///
    /// - If the account is not registered.
    fn force_unregister_storage_account(
        &mut self,
        account_id: &AccountIdRef,
    ) -> Result<NearToken, StorageForceUnregisterError>;

    /// Returns the storage balance bounds for the contract.
    fn get_storage_balance_bounds(&self) -> StorageBalanceBounds;

    /// Sets the storage balance bounds for the contract.
    fn set_storage_balance_bounds(&mut self, bounds: &StorageBalanceBounds);

    /// Convenience method for performing storage accounting, to be used after
    /// storage writes that are to be debited from the account's balance.
    ///
    /// # Errors
    ///
    /// See: [`lock_storage`](Nep145Controller::lock_storage) and [`unlock_storage`](Nep145Controller::unlock_storage).
    fn storage_accounting(
        &mut self,
        account_id: &AccountIdRef,
        storage_usage_start: u64,
    ) -> Result<(), StorageAccountingError> {
        let storage_usage_end = env::storage_usage();

        match storage_usage_end.cmp(&storage_usage_start) {
            Ordering::Equal => {}
            Ordering::Greater => {
                let storage_consumed = storage_usage_end - storage_usage_start;
                let storage_fee = env::storage_byte_cost()
                    .checked_mul(u128::from(storage_consumed))
                    .unwrap_or_else(|| env::panic_str(PANIC_MESSAGE_STORAGE_FEE_OVERFLOW));

                Nep145Controller::lock_storage(self, account_id, storage_fee)?;
            }
            Ordering::Less => {
                let storage_released = storage_usage_start - storage_usage_end;
                let storage_credit = env::storage_byte_cost()
                    .checked_mul(u128::from(storage_released))
                    .unwrap_or_else(|| env::panic_str(PANIC_MESSAGE_STORAGE_CREDIT_OVERFLOW));

                Nep145Controller::unlock_storage(self, account_id, storage_credit)?;
            }
        }

        Ok(())
    }
}

impl<T: Nep145ControllerInternal> Nep145Controller for T {
    type ForceUnregisterHook = <Self as Nep145ControllerInternal>::ForceUnregisterHook;

    fn get_storage_balance(
        &self,
        account_id: &AccountIdRef,
    ) -> Result<StorageBalance, AccountNotRegisteredError> {
        Self::slot_account(account_id)
            .read()
            .ok_or_else(|| AccountNotRegisteredError(account_id.to_owned()))
    }

    fn lock_storage(
        &mut self,
        account_id: &AccountIdRef,
        amount: NearToken,
    ) -> Result<StorageBalance, StorageLockError> {
        let mut account_slot = Self::slot_account(account_id);
        let mut balance = account_slot
            .read()
            .ok_or(AccountNotRegisteredError(account_id.to_owned()))?;

        balance.available =
            balance
                .available
                .checked_sub(amount)
                .ok_or(InsufficientBalanceError {
                    account_id: account_id.to_owned(),
                    attempted_to_use: amount,
                    available: balance.available,
                })?;

        account_slot.write(&balance);

        Ok(balance)
    }

    fn unlock_storage(
        &mut self,
        account_id: &AccountIdRef,
        amount: NearToken,
    ) -> Result<StorageBalance, StorageUnlockError> {
        let mut account_slot = Self::slot_account(account_id);

        let mut balance = account_slot
            .read()
            .ok_or(AccountNotRegisteredError(account_id.to_owned()))?;

        balance.available = {
            let new_available = balance
                .available
                .checked_add(amount)
                .unwrap_or_else(|| env::panic_str(PANIC_MESSAGE_STORAGE_AVAILABLE_OVERFLOW));

            if new_available > balance.total {
                return Err(ExcessiveUnlockError(account_id.to_owned()).into());
            }

            new_available
        };

        account_slot.write(&balance);

        Ok(balance)
    }

    fn deposit_to_storage_account(
        &mut self,
        account_id: &AccountIdRef,
        amount: NearToken,
    ) -> Result<StorageBalance, StorageDepositError> {
        let mut account_slot = Self::slot_account(account_id);

        let mut balance = account_slot.read().unwrap_or_default();

        balance.total = {
            let new_total = balance
                .total
                .checked_add(amount)
                .unwrap_or_else(|| env::panic_str(PANIC_MESSAGE_STORAGE_TOTAL_OVERFLOW));

            let bounds = self.get_storage_balance_bounds();

            if new_total < bounds.min {
                return Err(MinimumBalanceUnderrunError {
                    account_id: account_id.to_owned(),
                    minimum_balance: bounds.min,
                }
                .into());
            }

            if let Some(maximum_balance) = bounds.max {
                if new_total > maximum_balance {
                    return Err(MaximumBalanceOverrunError {
                        account_id: account_id.to_owned(),
                        maximum_balance,
                    }
                    .into());
                }
            }

            new_total
        };

        balance.available = balance
            .available
            .checked_add(amount)
            .unwrap_or_else(|| env::panic_str(PANIC_MESSAGE_INCONSISTENT_STATE_AVAILABLE));

        account_slot.write(&balance);

        Ok(balance)
    }

    fn withdraw_from_storage_account(
        &mut self,
        account_id: &AccountIdRef,
        amount: NearToken,
    ) -> Result<StorageBalance, StorageWithdrawError> {
        let mut account_slot = Self::slot_account(account_id);

        let mut balance = account_slot
            .read()
            .ok_or_else(|| AccountNotRegisteredError(account_id.to_owned()))?;

        balance.available =
            balance
                .available
                .checked_sub(amount)
                .ok_or_else(|| InsufficientBalanceError {
                    account_id: account_id.to_owned(),
                    available: balance.available,
                    attempted_to_use: amount,
                })?;

        balance.total = {
            let bounds = self.get_storage_balance_bounds();

            balance
                .total
                .checked_sub(amount)
                .filter(|&new_total| new_total >= bounds.min)
                .ok_or(MinimumBalanceUnderrunError {
                    account_id: account_id.to_owned(),
                    minimum_balance: bounds.min,
                })?
        };

        account_slot.write(&balance);

        Ok(balance)
    }

    fn unregister_storage_account(
        &mut self,
        account_id: &AccountIdRef,
    ) -> Result<NearToken, StorageUnregisterError> {
        let mut account_slot = Self::slot_account(account_id);

        let balance = account_slot
            .read()
            .ok_or_else(|| AccountNotRegisteredError(account_id.to_owned()))?;

        match balance.total.checked_sub(balance.available) {
            Some(locked_balance) if !locked_balance.is_zero() => {
                return Err(UnregisterWithLockedBalanceError {
                    account_id: account_id.to_owned(),
                    locked_balance,
                }
                .into());
            }
            None => env::panic_str(PANIC_MESSAGE_INCONSISTENT_STATE_AVAILABLE),
            _ => {}
        }

        account_slot.remove();

        Ok(balance.total)
    }

    fn force_unregister_storage_account(
        &mut self,
        account_id: &AccountIdRef,
    ) -> Result<NearToken, StorageForceUnregisterError> {
        let mut account_slot = Self::slot_account(account_id);

        let balance = account_slot
            .read()
            .ok_or_else(|| AccountNotRegisteredError(account_id.to_owned()))?;

        let action = Nep145ForceUnregister {
            account_id: account_id.into(),
            balance,
        };

        Self::ForceUnregisterHook::hook(self, &action, |_| {
            account_slot.remove();
        });

        Ok(action.balance.available)
    }

    fn get_storage_balance_bounds(&self) -> StorageBalanceBounds {
        Self::slot_balance_bounds().read().unwrap_or_default()
    }

    fn set_storage_balance_bounds(&mut self, bounds: &StorageBalanceBounds) {
        Self::slot_balance_bounds().write(bounds);
    }
}
