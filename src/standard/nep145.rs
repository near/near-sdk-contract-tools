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

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct StorageBalance {
    total: U128,
    available: U128,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct StorageBalanceBounds {
    min: U128,
    max: Option<U128>,
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
        "Account {account_id} has insufficient balance: {} available, {} required", required.0, available.0
    )]
pub struct InsufficientBalanceError {
    account_id: AccountId,
    required: U128,
    available: U128,
}

#[derive(Debug, Error)]
#[error("Account {0} is not registered")]
pub struct AccountNotRegisteredError(AccountId);

#[derive(Debug, Error)]
#[error("")]
pub struct ExcessiveUnlock(AccountId);

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
}

pub trait Nep145Controller {
    fn storage_balance(&self, account_id: &AccountId) -> Option<StorageBalance>;

    fn storage_balance_lock(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageLockError>;

    fn storage_balance_unlock(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageUnlockError>;

    fn storage_balance_deposit(&mut self, account_id: &AccountId, amount: U128) -> StorageBalance;
}

impl<T: Nep145ControllerInternal> Nep145Controller for T {
    fn storage_balance(&self, account_id: &AccountId) -> Option<StorageBalance> {
        Self::slot_account(account_id).read()
    }

    fn storage_balance_lock(
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
                required: amount,
                available: balance.available,
            })?
            .into();

        account_slot.write(&balance);

        Ok(balance)
    }

    fn storage_balance_unlock(
        &mut self,
        account_id: &AccountId,
        amount: U128,
    ) -> Result<StorageBalance, StorageUnlockError> {
        let mut account_slot = Self::slot_account(account_id);

        let mut balance = account_slot
            .read()
            .ok_or(AccountNotRegisteredError(account_id.clone()))?;

        balance.available = balance
            .available
            .0
            .checked_add(amount.0)
            .unwrap_or_else(|| env::panic_str("storage balance overflow"))
            .into();

        account_slot.write(&balance);

        Ok(balance)
    }

    fn storage_balance_deposit(&mut self, account_id: &AccountId, amount: U128) -> StorageBalance {
        todo!()
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
