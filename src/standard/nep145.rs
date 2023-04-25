//! NEP-145 Storage Management
//! <https://github.com/near/NEPs/blob/master/neps/nep-0145.md>
#![allow(missing_docs)] // ext_contract doesn't play nice with #![warn(missing_docs)]

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, ext_contract,
    json_types::U128,
    AccountId, BorshStorageKey,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{slot::Slot, DefaultStorageKey};

const PANIC_MESSAGE_STORAGE_TOTAL_OVERFLOW: &str = "storage total balance overflow";
const PANIC_MESSAGE_STORAGE_AVAILABLE_OVERFLOW: &str = "storage available balance overflow";
const PANIC_MESSAGE_INCONSISTENT_STATE_AVAILABLE: &str =
    "inconsistent state: available storage balance greater than total storage balance";

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
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

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
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

pub trait Nep145ControllerInternal {
    /// Root storage slot
    fn root() -> Slot<()> {
        Slot::new(DefaultStorageKey::Nep145)
    }

    fn slot_balance_bounds() -> Slot<StorageBalanceBounds> {
        Slot::new(StorageKey::BalanceBounds)
    }

    fn slot_account(account_id: &AccountId) -> Slot<StorageBalance> {
        Slot::new(StorageKey::Account(account_id))
    }
}

#[derive(Debug, Error)]
#[error(
    "Account {account_id} has insufficient balance: {} available, but attempted to lock {}", available.0, attempted_to_lock.0
)]
pub struct InsufficientBalanceError {
    account_id: AccountId,
    available: U128,
    attempted_to_lock: U128,
}

#[derive(Debug, Error)]
#[error("Account {0} is not registered")]
pub struct AccountNotRegisteredError(AccountId);

#[derive(Debug, Error)]
#[error("Account {0} cannot unlock more tokens than it has deposited")]
pub struct ExcessiveUnlockError(AccountId);

#[derive(Debug, Error)]
#[error("Account {account_id} must cover the minimum balance {}", minimum_balance.0)]
pub struct MinimumBalanceUnderrunError {
    account_id: AccountId,
    minimum_balance: U128,
}

#[derive(Debug, Error)]
#[error("Account {account_id} cannot unregister with locked balance {} > 0", locked_balance.0)]
pub struct UnregisterWithLockedBalanceError {
    account_id: AccountId,
    locked_balance: U128,
}

#[derive(Debug, Error)]
pub enum StorageLockError {
    #[error(transparent)]
    AccountNotRegistered(#[from] AccountNotRegisteredError),
    #[error(transparent)]
    InsufficientBalance(#[from] InsufficientBalanceError),
}

#[derive(Debug, Error)]
pub enum StorageUnlockError {
    #[error(transparent)]
    AccountNotRegistered(#[from] AccountNotRegisteredError),
    #[error(transparent)]
    ExcessiveUnlock(#[from] ExcessiveUnlockError),
}

#[derive(Debug, Error)]
pub enum StorageDepositError {
    #[error(transparent)]
    MinimumBalanceUnderrun(#[from] MinimumBalanceUnderrunError),
}

#[derive(Debug, Error)]
pub enum StorageWithdrawError {
    #[error(transparent)]
    AccountNotRegistered(#[from] AccountNotRegisteredError),
    #[error(transparent)]
    MinimumBalanceUnderrun(#[from] MinimumBalanceUnderrunError),
}

#[derive(Debug, Error)]
pub enum StorageUnregisterError {
    #[error(transparent)]
    AccountNotRegistered(#[from] AccountNotRegisteredError),
    #[error(transparent)]
    UnregisterWithLockedBalance(#[from] UnregisterWithLockedBalanceError),
}

#[derive(Debug, Error)]
pub enum StorageForceUnregisterError {
    #[error(transparent)]
    AccountNotRegistered(#[from] AccountNotRegisteredError),
}

pub trait Nep145Controller {
    fn storage_balance(&self, account_id: &AccountId) -> Option<StorageBalance>;

    fn storage_lock(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageLockError>;

    fn storage_unlock(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageUnlockError>;

    fn storage_deposit(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageDepositError>;

    fn storage_withdraw(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageWithdrawError>;

    fn storage_unregister(
        &mut self,
        account_id: &AccountId,
    ) -> Result<U128, StorageUnregisterError>;

    fn storage_force_unregister(
        &mut self,
        account_id: &AccountId,
    ) -> Result<U128, StorageForceUnregisterError>;

    fn storage_balance_bounds(&self) -> StorageBalanceBounds;
}

impl<T: Nep145ControllerInternal> Nep145Controller for T {
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

        Ok(balance.available)
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        Self::slot_balance_bounds().read().unwrap_or_default()
    }
}

#[ext_contract(ext_nep145)]
pub trait Nep145 {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance;

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance;

    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool;

    // read-only methods

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance>;

    fn storage_balance_bounds(&self) -> StorageBalanceBounds;
}
