//! NEP-141 fungible token core implementation
//! <https://github.com/near/NEPs/blob/master/neps/nep-0141.md>

use std::borrow::Cow;

use near_sdk::{AccountIdRef, BorshStorageKey, Gas, borsh::BorshSerialize, near};

use crate::{DefaultStorageKey, hook::Hook, slot::Slot, standard::nep297::*};

mod error;
pub use error::*;
mod event;
pub use event::*;
mod ext;
pub use ext::*;
pub mod hooks;

/// Gas value required for [`Nep141Resolver::ft_resolve_transfer`] call,
/// independent of the amount of gas required for the preceding
/// [`Nep141::ft_transfer`] call.
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas::from_gas(5_000_000_000_000);
/// Gas value required for [`Nep141::ft_transfer_call`] calls (includes gas for
/// the subsequent [`Nep141Resolver::ft_resolve_transfer`] call).
pub const GAS_FOR_FT_TRANSFER_CALL: Gas =
    Gas::from_gas(25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.as_gas());
/// Error message for insufficient gas.
pub const MORE_GAS_FAIL_MESSAGE: &str = "Insufficient gas attached.";

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey<'a> {
    TotalSupply,
    Account(&'a AccountIdRef),
}

/// Transfer metadata generic over both types of transfer (`ft_transfer` and
/// `ft_transfer_call`).
#[derive(PartialEq, Eq, Clone, Debug)]
#[near]
pub struct Nep141Transfer<'a> {
    /// Sender's account ID.
    pub sender_id: Cow<'a, AccountIdRef>,
    /// Receiver's account ID.
    pub receiver_id: Cow<'a, AccountIdRef>,
    /// Transferred amount.
    pub amount: u128,
    /// Optional memo string.
    pub memo: Option<Cow<'a, str>>,
    /// Message passed to contract located at `receiver_id`.
    pub msg: Option<Cow<'a, str>>,
    /// Is this transfer a revert as a result of a [`Nep141::ft_transfer_call`] -> [`Nep141Receiver::ft_on_transfer`] call?
    pub revert: bool,
}

impl<'a> Nep141Transfer<'a> {
    /// Create a new transfer action.
    pub fn new(
        amount: u128,
        sender_id: impl Into<Cow<'a, AccountIdRef>>,
        receiver_id: impl Into<Cow<'a, AccountIdRef>>,
    ) -> Self {
        Self {
            sender_id: sender_id.into(),
            receiver_id: receiver_id.into(),
            amount,
            memo: None,
            msg: None,
            revert: false,
        }
    }

    /// Add a memo string.
    #[must_use]
    pub fn memo(self, memo: impl Into<Cow<'a, str>>) -> Self {
        Self {
            memo: Some(memo.into()),
            ..self
        }
    }

    /// Add a message string.
    #[must_use]
    pub fn msg(self, msg: impl Into<Cow<'a, str>>) -> Self {
        Self {
            msg: Some(msg.into()),
            ..self
        }
    }

    /// Returns `true` if this transfer comes from a `ft_transfer_call`
    /// call, `false` otherwise.
    #[must_use]
    pub fn is_transfer_call(&self) -> bool {
        self.msg.is_some()
    }
}

/// Describes a mint operation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[near]
pub struct Nep141Mint<'a> {
    /// Amount to mint.
    pub amount: u128,
    /// Account ID to mint to.
    pub receiver_id: Cow<'a, AccountIdRef>,
    /// Optional memo string.
    pub memo: Option<Cow<'a, str>>,
}

impl<'a> Nep141Mint<'a> {
    /// Create a new mint action.
    pub fn new(amount: u128, receiver_id: impl Into<Cow<'a, AccountIdRef>>) -> Self {
        Self {
            amount,
            receiver_id: receiver_id.into(),
            memo: None,
        }
    }

    /// Add a memo string.
    #[must_use]
    pub fn memo(self, memo: impl Into<Cow<'a, str>>) -> Self {
        Self {
            memo: Some(memo.into()),
            ..self
        }
    }
}

/// Describes a burn operation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[near]
pub struct Nep141Burn<'a> {
    /// Amount to burn.
    pub amount: u128,
    /// Account ID to burn from.
    pub owner_id: Cow<'a, AccountIdRef>,
    /// Optional memo string.
    pub memo: Option<Cow<'a, str>>,
}

impl<'a> Nep141Burn<'a> {
    /// Create a new burn action.
    pub fn new(amount: u128, owner_id: impl Into<Cow<'a, AccountIdRef>>) -> Self {
        Self {
            amount,
            owner_id: owner_id.into(),
            memo: None,
        }
    }

    /// Add a memo string.
    #[must_use]
    pub fn memo(self, memo: impl Into<Cow<'a, str>>) -> Self {
        Self {
            memo: Some(memo.into()),
            ..self
        }
    }
}

/// Internal functions for [`Nep141Controller`]. Using these methods may result in unexpected behavior.
pub trait Nep141ControllerInternal {
    /// Hook for mint operations.
    type MintHook: for<'a> Hook<Self, Nep141Mint<'a>>
    where
        Self: Sized;
    /// Hook for transfer operations.
    type TransferHook: for<'a> Hook<Self, Nep141Transfer<'a>>
    where
        Self: Sized;
    /// Hook for burn operations.
    type BurnHook: for<'a> Hook<Self, Nep141Burn<'a>>
    where
        Self: Sized;

    /// Root storage slot.
    #[must_use]
    fn root() -> Slot<()> {
        Slot::new(DefaultStorageKey::Nep141)
    }

    /// Slot for account data.
    #[must_use]
    fn slot_account(account_id: &AccountIdRef) -> Slot<u128> {
        Self::root().field(StorageKey::Account(account_id))
    }

    /// Slot for storing total supply.
    #[must_use]
    fn slot_total_supply() -> Slot<u128> {
        Self::root().field(StorageKey::TotalSupply)
    }
}

/// Non-public implementations of functions for managing a fungible token.
pub trait Nep141Controller {
    /// Hook for mint operations.
    type MintHook: for<'a> Hook<Self, Nep141Mint<'a>>
    where
        Self: Sized;
    /// Hook for transfer operations.
    type TransferHook: for<'a> Hook<Self, Nep141Transfer<'a>>
    where
        Self: Sized;
    /// Hook for burn operations.
    type BurnHook: for<'a> Hook<Self, Nep141Burn<'a>>
    where
        Self: Sized;

    /// Get the balance of an account. Returns 0 if the account does not exist.
    fn balance_of(&self, account_id: &AccountIdRef) -> u128;

    /// Get the total circulating supply of the token.
    fn total_supply(&self) -> u128;

    /// Removes tokens from an account and decreases total supply. No event
    /// emission or hook invocation.
    ///
    /// # Errors
    ///
    /// - Account balance underflow.
    /// - Total supply underflow.
    fn withdraw_unchecked(
        &mut self,
        account_id: &AccountIdRef,
        amount: u128,
    ) -> Result<(), WithdrawError>;

    /// Increases the token balance of an account. Updates total supply. No
    /// event emission or hook invocation.
    ///
    /// # Errors
    ///
    /// - Account balance overflow.
    /// - Total supply overflow.
    fn deposit_unchecked(
        &mut self,
        account_id: &AccountIdRef,
        amount: u128,
    ) -> Result<(), DepositError>;

    /// Decreases the balance of `sender_account_id` by `amount` and increases
    /// the balance of `receiver_account_id` by the same. No change to total
    /// supply. No event emission or hook invocation.
    ///
    /// # Errors
    ///
    /// - Receiver balance overflow.
    /// - Sender balance underflow.
    fn transfer_unchecked(
        &mut self,
        sender_account_id: &AccountIdRef,
        receiver_account_id: &AccountIdRef,
        amount: u128,
    ) -> Result<(), TransferError>;

    /// Performs an NEP-141 token transfer, with event emission. Invokes
    /// [`Nep141Controller::TransferHook`].
    ///
    /// # Errors
    ///
    /// - Receiver balance overflow.
    /// - Sender balance underflow.
    fn transfer(&mut self, transfer: &Nep141Transfer<'_>) -> Result<(), TransferError>;

    /// Performs an NEP-141 token mint, with event emission. Invokes
    /// [`Nep141Controller::MintHook`].
    ///
    /// # Errors
    ///
    /// - Account balance overflow.
    /// - Total supply overflow.
    fn mint(&mut self, mint: &Nep141Mint<'_>) -> Result<(), DepositError>;

    /// Performs an NEP-141 token burn, with event emission. Invokes
    /// [`Nep141Controller::BurnHook`].
    ///
    /// # Errors
    ///
    /// - Account balance underflow.
    /// - Total supply underflow.
    fn burn(&mut self, burn: &Nep141Burn<'_>) -> Result<(), WithdrawError>;
}

impl<T: Nep141ControllerInternal> Nep141Controller for T {
    type MintHook = T::MintHook;
    type TransferHook = T::TransferHook;
    type BurnHook = T::BurnHook;

    fn balance_of(&self, account_id: &AccountIdRef) -> u128 {
        Self::slot_account(account_id).read().unwrap_or(0)
    }

    fn total_supply(&self) -> u128 {
        Self::slot_total_supply().read().unwrap_or(0)
    }

    fn withdraw_unchecked(
        &mut self,
        account_id: &AccountIdRef,
        amount: u128,
    ) -> Result<(), WithdrawError> {
        if amount != 0 {
            let balance = self.balance_of(account_id);
            if let Some(balance) = balance.checked_sub(amount) {
                Self::slot_account(account_id).write(&balance);
            } else {
                return Err(BalanceUnderflowError {
                    account_id: account_id.to_owned(),
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
        account_id: &AccountIdRef,
        amount: u128,
    ) -> Result<(), DepositError> {
        if amount != 0 {
            let balance = self.balance_of(account_id);
            if let Some(balance) = balance.checked_add(amount) {
                Self::slot_account(account_id).write(&balance);
            } else {
                return Err(BalanceOverflowError {
                    account_id: account_id.to_owned(),
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
        sender_account_id: &AccountIdRef,
        receiver_account_id: &AccountIdRef,
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
                    account_id: receiver_account_id.to_owned(),
                    balance: receiver_balance,
                    amount,
                }
                .into());
            }
        } else {
            return Err(BalanceUnderflowError {
                account_id: sender_account_id.to_owned(),
                balance: sender_balance,
                amount,
            }
            .into());
        }

        Ok(())
    }

    fn transfer(&mut self, transfer: &Nep141Transfer<'_>) -> Result<(), TransferError> {
        Self::TransferHook::hook(self, transfer, |contract| {
            contract.transfer_unchecked(
                &transfer.sender_id,
                &transfer.receiver_id,
                transfer.amount,
            )?;

            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: transfer.sender_id.clone(),
                new_owner_id: transfer.receiver_id.clone(),
                amount: transfer.amount.into(),
                memo: transfer.memo.clone(),
            }])
            .emit();

            Ok(())
        })
    }

    fn mint(&mut self, mint: &Nep141Mint) -> Result<(), DepositError> {
        Self::MintHook::hook(self, mint, |contract| {
            contract.deposit_unchecked(&mint.receiver_id, mint.amount)?;

            Nep141Event::FtMint(vec![FtMintData {
                owner_id: mint.receiver_id.clone(),
                amount: mint.amount.into(),
                memo: mint.memo.clone(),
            }])
            .emit();

            Ok(())
        })
    }

    fn burn(&mut self, burn: &Nep141Burn) -> Result<(), WithdrawError> {
        Self::BurnHook::hook(self, burn, |contract| {
            contract.withdraw_unchecked(&burn.owner_id, burn.amount)?;

            Nep141Event::FtBurn(vec![FtBurnData {
                owner_id: burn.owner_id.clone(),
                amount: burn.amount.into(),
                memo: burn.memo.clone(),
            }])
            .emit();

            Ok(())
        })
    }
}
