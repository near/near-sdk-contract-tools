//! NEP-245 fungible token core implementation
//! <https://github.com/near/NEPs/blob/master/neps/nep-0245.md>

use std::borrow::Cow;

use near_sdk::{
    borsh::BorshSerialize, collections::UnorderedSet, near, AccountIdRef, BorshStorageKey, Gas,
};

use crate::{hook::Hook, slot::Slot, standard::nep297::*, DefaultStorageKey};

mod error;
pub use error::*;
mod event;
pub use event::*;
mod ext;
pub use ext::*;
pub mod hooks;

pub type ApprovalId = u32;
pub type TokenId = String;
pub type TokenIdRef = str;

/// Gas value required for [`Nep245Resolver::ft_resolve_transfer`] call,
/// independent of the amount of gas required for the preceding
/// [`Nep245::ft_transfer`] call.
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas::from_gas(5_000_000_000_000);
/// Gas value required for [`Nep245::ft_transfer_call`] calls (includes gas for
/// the subsequent [`Nep245Resolver::ft_resolve_transfer`] call).
pub const GAS_FOR_FT_TRANSFER_CALL: Gas =
    Gas::from_gas(25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.as_gas());
/// Error message for insufficient gas.
pub const MORE_GAS_FAIL_MESSAGE: &str = "Insufficient gas attached.";

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey<'a> {
    Tokens,
    Supply(&'a TokenIdRef),
    Balance(&'a TokenIdRef, &'a AccountIdRef),
}

#[derive(PartialEq, Eq, Debug, Clone)]
#[near]
pub struct TransferApproval<'a> {
    pub owner_id: Cow<'a, AccountIdRef>,
    pub approval_id: ApprovalId,
    pub amount: u128,
}

impl<'a> From<&'a MtResolveTransferApproval> for TransferApproval<'a> {
    fn from(value: &'a MtResolveTransferApproval) -> Self {
        Self {
            owner_id: value.owner_id().into(),
            approval_id: value.approval_id(),
            amount: value.amount(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
#[near]
pub struct TokenAmount<'a> {
    pub token_id: Cow<'a, TokenIdRef>,
    pub amount: u128,
    // pub approval: Option<TransferApproval<'a>>,
}

/// Transfer metadata generic over both types of transfer (`ft_transfer` and
/// `ft_transfer_call`).
#[derive(PartialEq, Eq, Clone, Debug)]
#[near]
pub struct Nep245Transfer<'a> {
    /// Sender's account ID.
    pub sender_id: Cow<'a, AccountIdRef>,
    /// Receiver's account ID.
    pub receiver_id: Cow<'a, AccountIdRef>,
    /// Transferred amounts.
    pub payload: Vec<TokenAmount<'a>>,
    /// Optional memo string.
    pub memo: Option<Cow<'a, str>>,
    /// Message passed to contract located at `receiver_id`.
    pub msg: Option<Cow<'a, str>>,
    /// Is this transfer a revert as a result of a [`Nep245::mt_transfer_call`] -> [`Nep245Receiver::mt_on_transfer`] call?
    pub revert: bool,
}

impl<'a> Nep245Transfer<'a> {
    // Create a new transfer action of no tokens.
    #[must_use]
    pub fn empty(
        capacity: usize,
        sender_id: impl Into<Cow<'a, AccountIdRef>>,
        receiver_id: impl Into<Cow<'a, AccountIdRef>>,
    ) -> Self {
        Self {
            receiver_id: receiver_id.into(),
            sender_id: sender_id.into(),
            payload: Vec::with_capacity(capacity),
            memo: None,
            msg: None,
            revert: false,
        }
    }

    /// Create a new transfer action of a single token.
    pub fn single(
        token_id: impl Into<Cow<'a, TokenIdRef>>,
        amount: u128,
        sender_id: impl Into<Cow<'a, AccountIdRef>>,
        receiver_id: impl Into<Cow<'a, AccountIdRef>>,
    ) -> Self {
        Self {
            receiver_id: receiver_id.into(),
            sender_id: sender_id.into(),
            payload: vec![TokenAmount {
                token_id: token_id.into(),
                amount,
                // approval: None,
            }],
            memo: None,
            msg: None,
            revert: false,
        }
    }

    pub fn and_transfer(
        mut self,
        token_id: impl Into<Cow<'a, TokenIdRef>>,
        amount: u128,
        // approval: Option<TransferApproval<'a>>,
    ) -> Self {
        self.payload.push(TokenAmount {
            token_id: token_id.into(),
            amount,
            // approval,
        });
        self
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
pub struct Nep245Mint<'a> {
    /// Amounts to mint.
    pub payload: Vec<TokenAmount<'a>>,
    /// Account ID to mint to.
    pub receiver_id: Cow<'a, AccountIdRef>,
    /// Optional memo string.
    pub memo: Option<Cow<'a, str>>,
}

impl<'a> Nep245Mint<'a> {
    /// Create a new mint action.
    pub fn single(
        token_id: impl Into<Cow<'a, TokenIdRef>>,
        amount: u128,
        receiver_id: impl Into<Cow<'a, AccountIdRef>>,
    ) -> Self {
        Self {
            payload: vec![TokenAmount {
                token_id: token_id.into(),
                amount,
            }],
            receiver_id: receiver_id.into(),
            memo: None,
        }
    }

    /// Add a memo string.
    #[must_use]
    pub fn memo(mut self, memo: impl Into<Cow<'a, str>>) -> Self {
        self.memo = Some(memo.into());
        self
    }
}

/// Describes a burn operation.
#[derive(Clone, Debug, PartialEq, Eq)]
#[near]
pub struct Nep245Burn<'a> {
    /// Amounts to burn.
    pub payload: Vec<TokenAmount<'a>>,
    /// Account ID to burn from.
    pub owner_id: Cow<'a, AccountIdRef>,
    /// Optional memo string.
    pub memo: Option<Cow<'a, str>>,
}

impl<'a> Nep245Burn<'a> {
    /// Create a new burn action with no burn actions.
    pub fn empty(capacity: usize, owner_id: impl Into<Cow<'a, AccountIdRef>>) -> Self {
        Self {
            payload: Vec::with_capacity(capacity),
            owner_id: owner_id.into(),
            memo: None,
        }
    }

    pub fn and_burn(mut self, token_id: impl Into<Cow<'a, TokenIdRef>>, amount: u128) -> Self {
        self.payload.push(TokenAmount {
            token_id: token_id.into(),
            amount,
        });
        self
    }

    /// Create a new burn action for burning an amount of a single token.
    pub fn single(
        token_id: impl Into<Cow<'a, TokenIdRef>>,
        amount: u128,
        owner_id: impl Into<Cow<'a, AccountIdRef>>,
    ) -> Self {
        Self {
            payload: vec![TokenAmount {
                token_id: token_id.into(),
                amount,
            }],
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

/// Internal functions for [`Nep245Controller`]. Using these methods may result in unexpected behavior.
pub trait Nep245ControllerInternal {
    /// Hook for mint operations.
    type MintHook: for<'a> Hook<Self, Nep245Mint<'a>>
    where
        Self: Sized;
    /// Hook for transfer operations.
    type TransferHook: for<'a> Hook<Self, Nep245Transfer<'a>>
    where
        Self: Sized;
    /// Hook for burn operations.
    type BurnHook: for<'a> Hook<Self, Nep245Burn<'a>>
    where
        Self: Sized;

    /// Root storage slot.
    #[must_use]
    fn root() -> Slot<()> {
        Slot::new(DefaultStorageKey::Nep245)
    }

    /// Root storage slot.
    #[must_use]
    fn slot_tokens() -> Slot<near_sdk::collections::UnorderedSet<TokenId>> {
        Self::root().field(StorageKey::Tokens)
    }

    #[must_use]
    fn read_tokens_set() -> UnorderedSet<TokenId> {
        Self::slot_tokens()
            .read()
            .unwrap_or_else(|| UnorderedSet::new(Self::slot_tokens().key))
    }

    /// Slot for account data.
    #[must_use]
    fn slot_balance(token_id: &TokenIdRef, account_id: &AccountIdRef) -> Slot<u128> {
        Self::root().field(StorageKey::Balance(token_id, account_id))
    }

    /// Slot for storing total supply.
    #[must_use]
    fn slot_supply(token_id: &TokenIdRef) -> Slot<u128> {
        Self::root().field(StorageKey::Supply(token_id))
    }
}

/// Non-public implementations of functions for managing a fungible token.
pub trait Nep245Controller {
    /// Hook for mint operations.
    type MintHook: for<'a> Hook<Self, Nep245Mint<'a>>
    where
        Self: Sized;
    /// Hook for transfer operations.
    type TransferHook: for<'a> Hook<Self, Nep245Transfer<'a>>
    where
        Self: Sized;
    /// Hook for burn operations.
    type BurnHook: for<'a> Hook<Self, Nep245Burn<'a>>
    where
        Self: Sized;

    fn create_token(&mut self, token_id: TokenId) -> Result<(), TokenIdCollisionError>;

    fn iter_tokens(&self) -> impl Iterator<Item = &TokenIdRef>;

    /// Get the balance of an account. Returns 0 if the account does not exist.
    fn balance_of(&self, token_id: &TokenIdRef, account_id: &AccountIdRef) -> u128;

    /// Get the total circulating supply of the token.
    fn supply(&self, token_id: &TokenIdRef) -> u128;

    /// Removes tokens from an account and decreases total supply. No event
    /// emission or hook invocation.
    ///
    /// # Errors
    ///
    /// - Account balance underflow.
    /// - Total supply underflow.
    fn withdraw_unchecked(
        &mut self,
        token_id: &TokenIdRef,
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
        token_id: &TokenIdRef,
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
        token_id: &TokenIdRef,
        sender_account_id: &AccountIdRef,
        receiver_account_id: &AccountIdRef,
        amount: u128,
    ) -> Result<(), TransferError>;

    /// Performs an NEP-245 token transfer, with event emission. Invokes
    /// [`Nep245Controller::TransferHook`].
    ///
    /// # Errors
    ///
    /// - Receiver balance overflow.
    /// - Sender balance underflow.
    fn transfer(&mut self, transfer: &Nep245Transfer<'_>) -> Result<(), TransferError>;

    /// Performs an NEP-245 token mint, with event emission. Invokes
    /// [`Nep245Controller::MintHook`].
    ///
    /// # Errors
    ///
    /// - Account balance overflow.
    /// - Total supply overflow.
    fn mint(&mut self, mint: &Nep245Mint<'_>) -> Result<(), DepositError>;

    /// Performs an NEP-245 token burn, with event emission. Invokes
    /// [`Nep245Controller::BurnHook`].
    ///
    /// # Errors
    ///
    /// - Account balance underflow.
    /// - Total supply underflow.
    fn burn(&mut self, burn: &Nep245Burn<'_>) -> Result<(), WithdrawError>;
}

impl<T: Nep245ControllerInternal> Nep245Controller for T {
    type MintHook = T::MintHook;
    type TransferHook = T::TransferHook;
    type BurnHook = T::BurnHook;

    fn create_token(&mut self, token_id: TokenId) -> Result<(), TokenIdCollisionError> {
        let mut tokens = Self::read_tokens_set();
        if tokens.insert(&token_id) {
            Self::slot_tokens().write(&tokens);
            Ok(())
        } else {
            Err(TokenIdCollisionError { token_id })
        }
    }

    fn iter_tokens(&self) -> impl Iterator<Item = &TokenIdRef> {
        Self::read_tokens_set().iter()
    }

    fn balance_of(&self, token_id: &TokenIdRef, account_id: &AccountIdRef) -> u128 {
        Self::slot_balance(token_id, account_id).read().unwrap_or(0)
    }

    fn supply(&self, token_id: &TokenIdRef) -> u128 {
        Self::slot_supply(token_id).read().unwrap_or(0)
    }

    fn withdraw_unchecked(
        &mut self,
        token_id: &TokenIdRef,
        account_id: &AccountIdRef,
        amount: u128,
    ) -> Result<(), WithdrawError> {
        if amount != 0 {
            let balance = self.balance_of(token_id, account_id);
            let balance = balance
                .checked_sub(amount)
                .ok_or_else(|| BalanceUnderflowError {
                    token_id: token_id.to_owned(),
                    account_id: account_id.to_owned(),
                    balance,
                    amount,
                })?;

            let supply = self.supply(token_id);
            let supply = supply
                .checked_sub(amount)
                .ok_or_else(|| SupplyUnderflowError {
                    token_id: token_id.to_owned(),
                    supply,
                    amount,
                })?;

            Self::slot_balance(token_id, account_id).write(&balance);
            Self::slot_supply(token_id).write(&supply);
        }

        Ok(())
    }

    fn deposit_unchecked(
        &mut self,
        token_id: &TokenIdRef,
        account_id: &AccountIdRef,
        amount: u128,
    ) -> Result<(), DepositError> {
        if amount != 0 {
            let balance = self.balance_of(token_id, account_id);
            let balance = balance
                .checked_add(amount)
                .ok_or_else(|| BalanceOverflowError {
                    token_id: token_id.to_owned(),
                    account_id: account_id.to_owned(),
                    balance,
                    amount,
                })?;

            let supply = self.supply(token_id);
            let supply = supply
                .checked_add(amount)
                .ok_or_else(|| SupplyOverflowError {
                    token_id: token_id.to_owned(),
                    supply,
                    amount,
                })?;
            Self::slot_balance(token_id, account_id).write(&balance);
            Self::slot_supply(token_id).write(&supply);
        }

        Ok(())
    }

    fn transfer_unchecked(
        &mut self,
        token_id: &TokenIdRef,
        sender_account_id: &AccountIdRef,
        receiver_account_id: &AccountIdRef,
        amount: u128,
    ) -> Result<(), TransferError> {
        let sender_balance = self.balance_of(token_id, sender_account_id);
        let sender_balance =
            sender_balance
                .checked_sub(amount)
                .ok_or_else(|| BalanceUnderflowError {
                    token_id: token_id.to_owned(),
                    account_id: sender_account_id.to_owned(),
                    balance: sender_balance,
                    amount,
                })?;

        let receiver_balance = self.balance_of(token_id, receiver_account_id);
        let receiver_balance =
            receiver_balance
                .checked_add(amount)
                .ok_or_else(|| BalanceOverflowError {
                    token_id: token_id.to_owned(),
                    account_id: receiver_account_id.to_owned(),
                    balance: receiver_balance,
                    amount,
                })?;

        Self::slot_balance(token_id, sender_account_id).write(&sender_balance);
        Self::slot_balance(token_id, receiver_account_id).write(&receiver_balance);

        Ok(())
    }

    fn transfer(&mut self, transfer: &Nep245Transfer<'_>) -> Result<(), TransferError> {
        Self::TransferHook::hook(self, transfer, |contract| {
            let mut token_ids = Vec::with_capacity(transfer.payload.len());
            let mut amounts = Vec::with_capacity(transfer.payload.len());
            for token in &transfer.payload {
                contract.transfer_unchecked(
                    &token.token_id,
                    &transfer.sender_id,
                    &transfer.receiver_id,
                    token.amount,
                )?;
                token_ids.push(token.token_id);
                amounts.push(token.amount.into());
            }

            Nep245Event::MtTransfer(vec![MtTransferData {
                authorized_id: None,
                old_owner_id: transfer.sender_id.clone(),
                new_owner_id: transfer.receiver_id.clone(),
                token_ids,
                amounts,
                memo: transfer.memo.clone(),
            }])
            .emit();

            Ok(())
        })
    }

    fn mint(&mut self, mint: &Nep245Mint) -> Result<(), DepositError> {
        Self::MintHook::hook(self, mint, |contract| {
            let mut token_ids = Vec::with_capacity(mint.payload.len());
            let mut amounts = Vec::with_capacity(mint.payload.len());

            for token in &mint.payload {
                contract.deposit_unchecked(&token.token_id, &mint.receiver_id, token.amount)?;
                token_ids.push(token.token_id);
                amounts.push(token.amount.into());
            }

            Nep245Event::MtMint(vec![MtMintData {
                owner_id: mint.receiver_id.clone(),
                token_ids,
                amounts,
                memo: mint.memo.clone(),
            }])
            .emit();

            Ok(())
        })
    }

    fn burn(&mut self, burn: &Nep245Burn) -> Result<(), WithdrawError> {
        Self::BurnHook::hook(self, burn, |contract| {
            let mut token_ids = Vec::with_capacity(burn.payload.len());
            let mut amounts = Vec::with_capacity(burn.payload.len());

            for token in &burn.payload {
                contract.withdraw_unchecked(&token.token_id, &burn.owner_id, token.amount)?;
                token_ids.push(token.token_id);
                amounts.push(token.amount.into());
            }

            Nep245Event::MtBurn(vec![MtBurnData {
                authorized_id: None,
                owner_id: burn.owner_id.clone(),
                token_ids,
                amounts,
                memo: burn.memo.clone(),
            }])
            .emit();

            Ok(())
        })
    }
}
