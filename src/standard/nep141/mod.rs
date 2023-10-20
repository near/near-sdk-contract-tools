//! NEP-141 fungible token core implementation
//! <https://github.com/near/NEPs/blob/master/neps/nep-0141.md>

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    AccountId, BorshStorageKey, Gas,
};
use serde::{Deserialize, Serialize};

use crate::{
    slot::Slot,
    standard::{nep145::*, nep297::*},
    utils::Hook,
    DefaultStorageKey,
};

mod error;
pub use error::*;
mod event;
pub use event::*;
mod ext;
pub use ext::*;

/// Gas value required for ft_resolve_transfer calls
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(5_000_000_000_000);
/// Gas value required for ft_transfer_call calls (includes gas for )
pub const GAS_FOR_FT_TRANSFER_CALL: Gas = Gas(25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.0);
/// Error message for insufficient gas.
pub const MORE_GAS_FAIL_MESSAGE: &str = "More gas is required";

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    TotalSupply,
    Account(AccountId),
}

/// Transfer metadata generic over both types of transfer (`ft_transfer` and
/// `ft_transfer_call`).
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone, Debug)]
pub struct Nep141Transfer {
    /// Sender's account ID.
    pub sender_id: AccountId,
    /// Receiver's account ID.
    pub receiver_id: AccountId,
    /// Transferred amount.
    pub amount: u128,
    /// Optional memo string.
    pub memo: Option<String>,
    /// Message passed to contract located at `receiver_id`.
    pub msg: Option<String>,
    /// Is this transfer a revert as a result of a [`Nep141::ft_transfer_call`] -> [`Nep141Receiver::ft_on_transfer`] call?
    pub revert: bool,
}

impl Nep141Transfer {
    /// Returns `true` if this transfer comes from a `ft_transfer_call`
    /// call, `false` otherwise.
    pub fn is_transfer_call(&self) -> bool {
        self.msg.is_some()
    }
}

pub struct Nep141Mint {
    pub amount: u128,
    pub account_id: AccountId,
    pub memo: Option<String>,
}

pub struct Nep141Burn {
    pub amount: u128,
    pub account_id: AccountId,
    pub memo: Option<String>,
}

/// Internal functions for [`Nep141Controller`]. Using these methods may result in unexpected behavior.
pub trait Nep141ControllerInternal {
    type MintHook: Hook<Self, Nep141Mint>
    where
        Self: Sized;
    type TransferHook: Hook<Self, Nep141Transfer>
    where
        Self: Sized;
    type BurnHook: Hook<Self, Nep141Burn>
    where
        Self: Sized;

    /// Root storage slot.
    fn root() -> Slot<()> {
        Slot::new(DefaultStorageKey::Nep141)
    }

    /// Slot for account data.
    fn slot_account(account_id: &AccountId) -> Slot<u128> {
        Self::root().field(StorageKey::Account(account_id.clone()))
    }

    /// Slot for storing total supply.
    fn slot_total_supply() -> Slot<u128> {
        Self::root().field(StorageKey::TotalSupply)
    }
}

/// Non-public implementations of functions for managing a fungible token.
pub trait Nep141Controller {
    type MintHook: Hook<Self, Nep141Mint>
    where
        Self: Sized;
    type TransferHook: Hook<Self, Nep141Transfer>
    where
        Self: Sized;
    type BurnHook: Hook<Self, Nep141Burn>
    where
        Self: Sized;

    /// Get the balance of an account. Returns 0 if the account does not exist.
    fn balance_of(&self, account_id: &AccountId) -> u128;

    /// Get the total circulating supply of the token.
    fn total_supply(&self) -> u128;

    /// Removes tokens from an account and decreases total supply. No event
    /// emission.
    fn withdraw_unchecked(
        &mut self,
        account_id: &AccountId,
        amount: u128,
    ) -> Result<(), WithdrawError>;

    /// Increases the token balance of an account. Updates total supply. No
    /// event emission.
    fn deposit_unchecked(
        &mut self,
        account_id: &AccountId,
        amount: u128,
    ) -> Result<(), DepositError>;

    /// Decreases the balance of `sender_account_id` by `amount` and increases
    /// the balance of `receiver_account_id` by the same. No change to total
    /// supply. No event emission.
    ///
    /// # Panics
    ///
    /// Panics if the balance of `sender_account_id` < `amount` or if the
    /// balance of `receiver_account_id` plus `amount` >= `u128::MAX`.
    fn transfer_unchecked(
        &mut self,
        sender_account_id: &AccountId,
        receiver_account_id: &AccountId,
        amount: u128,
    ) -> Result<(), TransferError>;

    /// Performs an NEP-141 token transfer, with event emission.
    ///
    /// # Panics
    ///
    /// See: [`Nep141Controller::transfer_unchecked`]
    fn transfer(&mut self, transfer: &Nep141Transfer) -> Result<(), TransferError>;

    /// Performs an NEP-141 token mint, with event emission.
    ///
    /// # Panics
    ///
    /// See: [`Nep141Controller::deposit_unchecked`]
    fn mint(&mut self, mint: &Nep141Mint) -> Result<(), DepositError>;

    /// Performs an NEP-141 token burn, with event emission.
    ///
    /// # Panics
    ///
    /// See: [`Nep141Controller::withdraw_unchecked`]
    fn burn(&mut self, mint: &Nep141Burn) -> Result<(), WithdrawError>;
}

impl<T: Nep141ControllerInternal> Nep141Controller for T {
    type MintHook = T::MintHook;
    type TransferHook = T::TransferHook;
    type BurnHook = T::BurnHook;

    fn balance_of(&self, account_id: &AccountId) -> u128 {
        Self::slot_account(account_id).read().unwrap_or(0)
    }

    fn total_supply(&self) -> u128 {
        Self::slot_total_supply().read().unwrap_or(0)
    }

    fn withdraw_unchecked(
        &mut self,
        account_id: &AccountId,
        amount: u128,
    ) -> Result<(), WithdrawError> {
        if amount != 0 {
            let balance = self.balance_of(account_id);
            if let Some(balance) = balance.checked_sub(amount) {
                Self::slot_account(account_id).write(&balance);
            } else {
                return Err(BalanceUnderflowError {
                    account_id: account_id.clone(),
                    balance,
                    amount,
                }
                .into());
            }

            let total_supply = self.total_supply();
            if let Some(total_supply) = total_supply.checked_sub(amount) {
                Self::slot_total_supply().write(&total_supply);
            } else {
                return Err(TotalSupplyUnderflowError {
                    total_supply,
                    amount,
                }
                .into());
            }
        }

        Ok(())
    }

    fn deposit_unchecked(
        &mut self,
        account_id: &AccountId,
        amount: u128,
    ) -> Result<(), DepositError> {
        if amount != 0 {
            let balance = self.balance_of(account_id);
            if let Some(balance) = balance.checked_add(amount) {
                Self::slot_account(account_id).write(&balance);
            } else {
                return Err(BalanceOverflowError {
                    account_id: account_id.clone(),
                    balance,
                    amount,
                }
                .into());
            }

            let total_supply = self.total_supply();
            if let Some(total_supply) = total_supply.checked_add(amount) {
                Self::slot_total_supply().write(&total_supply);
            } else {
                return Err(TotalSupplyOverflowError {
                    total_supply,
                    amount,
                }
                .into());
            }
        }

        Ok(())
    }

    fn transfer_unchecked(
        &mut self,
        sender_account_id: &AccountId,
        receiver_account_id: &AccountId,
        amount: u128,
    ) -> Result<(), TransferError> {
        let sender_balance = self.balance_of(sender_account_id);

        if let Some(sender_balance) = sender_balance.checked_sub(amount) {
            let receiver_balance = self.balance_of(receiver_account_id);
            if let Some(receiver_balance) = receiver_balance.checked_add(amount) {
                Self::slot_account(sender_account_id).write(&sender_balance);
                Self::slot_account(receiver_account_id).write(&receiver_balance);
            } else {
                return Err(BalanceOverflowError {
                    account_id: receiver_account_id.clone(),
                    balance: receiver_balance,
                    amount,
                }
                .into());
            }
        } else {
            return Err(BalanceUnderflowError {
                account_id: sender_account_id.clone(),
                balance: sender_balance,
                amount,
            }
            .into());
        }

        Ok(())
    }

    fn transfer(&mut self, transfer: &Nep141Transfer) -> Result<(), TransferError> {
        let state = Self::TransferHook::before(self, transfer);

        self.transfer_unchecked(&transfer.sender_id, &transfer.receiver_id, transfer.amount)?;

        Nep141Event::FtTransfer(vec![FtTransferData {
            old_owner_id: transfer.sender_id.clone(),
            new_owner_id: transfer.receiver_id.clone(),
            amount: transfer.amount.into(),
            memo: transfer.memo.clone(),
        }])
        .emit();

        Self::TransferHook::after(self, transfer, state);

        Ok(())
    }

    fn mint(&mut self, mint: &Nep141Mint) -> Result<(), DepositError> {
        let state = Self::MintHook::before(self, mint);

        self.deposit_unchecked(&mint.account_id, mint.amount)?;

        Self::MintHook::after(self, mint, state);

        Nep141Event::FtMint(vec![FtMintData {
            owner_id: mint.account_id.clone(),
            amount: mint.amount.into(),
            memo: mint.memo.clone(),
        }])
        .emit();

        Ok(())
    }

    fn burn(&mut self, burn: &Nep141Burn) -> Result<(), WithdrawError> {
        let state = Self::BurnHook::before(self, burn);

        self.withdraw_unchecked(&burn.account_id, burn.amount)?;

        Self::BurnHook::after(self, burn, state);

        Nep141Event::FtBurn(vec![FtBurnData {
            owner_id: burn.account_id.clone(),
            amount: burn.amount.into(),
            memo: burn.memo.clone(),
        }])
        .emit();

        Ok(())
    }
}

pub mod hooks {
    use crate::{utils::Hook, standard::nep145::Nep145ForceUnregister};

    use super::{Nep141Controller, Nep141ControllerInternal, Nep141Burn};

    pub struct BurnOnForceUnregisterHook;

    impl<C: Nep141Controller + Nep141ControllerInternal> Hook<C, Nep145ForceUnregister<'_>>
        for BurnOnForceUnregisterHook
    {
        fn before(_contract: &C, _args: &Nep145ForceUnregister<'_>) -> Self {
            Self
        }

        fn after(contract: &mut C, args: &Nep145ForceUnregister<'_>, _: Self) {
            let balance = contract.balance_of(args.account_id);
            contract
                .burn(&Nep141Burn {
                    amount: balance,
                    account_id: args.account_id.clone(),
                    memo: Some("storage forced unregistration".to_string()),
                })
                .unwrap_or_else(|e| {
                    near_sdk::env::panic_str(&format!(
                        "Failed to burn tokens during forced unregistration: {e}",
                    ))
                });

            <C as Nep141ControllerInternal>::slot_account(args.account_id).remove();
        }
    }
}
