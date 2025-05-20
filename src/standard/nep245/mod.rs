//! NEP-245 fungible token core implementation
//! <https://github.com/near/NEPs/blob/master/neps/nep-0245.md>

use std::{
    borrow::Cow,
    iter::{IntoIterator, Iterator},
};

use near_sdk::{
    borsh::BorshSerialize, collections::Vector, env, json_types::U128, near, AccountId,
    AccountIdRef, BorshStorageKey, Gas, Promise,
};

use crate::{hook::Hook, slot::Slot, standard::nep297::*, DefaultStorageKey};

mod error;
pub use error::*;
mod event;
pub use event::*;
mod ext;
pub use ext::*;
pub mod hooks;

/// Type of an approval ID.
pub type ApprovalId = u32;
/// Type of a token ID.
pub type TokenId = String;
/// Reference type of a token ID.
pub type TokenIdRef = str;

// TODO: Measure gas values.

/// Gas value required for [`Nep245Resolver::mt_resolve_transfer`] call,
/// independent of the amount of gas required for the preceding
/// [`Nep245::mt_transfer`] call.
pub const GAS_FOR_MT_RESOLVE_TRANSFER: Gas = Gas::from_gas(5_000_000_000_000);
/// Gas value required for [`Nep245::mt_transfer_call`] calls (includes gas for
/// the subsequent [`Nep245Resolver::mt_resolve_transfer`] call).
pub const GAS_FOR_MT_TRANSFER_CALL: Gas =
    Gas::from_gas(25_000_000_000_000).saturating_add(GAS_FOR_MT_RESOLVE_TRANSFER);
/// Error message for insufficient gas.
pub const INSUFFICIENT_GAS_MESSAGE: &str = "Insufficient gas attached.";

#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey<'a> {
    Tokens,
    Token(&'a TokenIdRef),
    Balance(&'a TokenIdRef, &'a AccountIdRef),
}

/// Token-wide metadata storage.
#[derive(PartialEq, Eq, Debug, Clone)]
#[near(serializers = [borsh])]
pub struct TokenRecord {
    /// Only `Some` if the supply has only ever been 1.
    /// (Will not be used even if tokens are burned from >1 to 1.)
    pub owner_id: Option<AccountId>,
    /// The quantity of tokens in circulation.
    pub supply: u128,
}

// #[derive(PartialEq, Eq, Debug, Clone)]
// #[near]
// pub struct TransferApproval<'a> {
//     pub owner_id: Cow<'a, AccountIdRef>,
//     pub approval_id: ApprovalId,
//     pub amount: u128,
// }

// impl<'a> From<&'a MtResolveTransferApproval> for TransferApproval<'a> {
//     fn from(value: &'a MtResolveTransferApproval) -> Self {
//         Self {
//             owner_id: value.owner_id().into(),
//             approval_id: value.approval_id(),
//             amount: value.amount(),
//         }
//     }
// }

/// A token amount involved in a mint, transfer, or burn action.
#[derive(PartialEq, Eq, Debug, Clone)]
#[near]
pub struct TokenAmount<'a> {
    /// The token ID involved.
    pub token_id: Cow<'a, TokenIdRef>,
    /// The amount of tokens.
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
    /// Is this transfer a revert as a result of a [`Nep245::mt_transfer_call`] -> [`Nep245Receiver::mt_on_transfer`] call?
    pub revert: bool,
}

impl<'a> Nep245Transfer<'a> {
    /// Create a new multi token transfer.
    ///
    /// The types of the arguments of this function are intended to closely
    /// replicate those in the NEP-245 standard functions so minimal
    /// transformation is required prior to invocation.
    ///
    /// # Errors
    ///
    /// - If the input iterators are of differing lengths.
    pub fn new(
        sender_id: impl Into<Cow<'a, AccountIdRef>>,
        receiver_id: impl Into<Cow<'a, AccountIdRef>>,
        count: usize,
        token_ids: impl IntoIterator<Item = impl Into<Cow<'a, TokenIdRef>>>,
        amounts: impl IntoIterator<Item = impl Into<u128>>,
        approvals: Option<impl IntoIterator<Item = Option<MtTransferApproval>>>,
        memo: Option<impl Into<Cow<'a, str>>>,
    ) -> Result<Self, LengthMismatchError> {
        let mut payload = Vec::with_capacity(count);
        let mut token_ids = token_ids.into_iter();
        let mut amounts = amounts.into_iter();
        let mut approvals: Option<_> = approvals.map(IntoIterator::into_iter);

        loop {
            match (
                token_ids.next(),
                amounts.next(),
                approvals.as_mut().map(Iterator::next),
            ) {
                (Some(token_id), Some(amount), None | Some(Some(_))) => payload.push(TokenAmount {
                    token_id: token_id.into(),
                    amount: amount.into(),
                }),
                (None, None, None) => break,
                _ => {
                    return Err(LengthMismatchError);
                }
            }
        }

        Ok(Self {
            sender_id: sender_id.into(),
            receiver_id: receiver_id.into(),
            payload,
            memo: memo.map(Into::into),
            revert: false,
        })
    }

    /// Create a new transfer action of no tokens.
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
            revert: false,
        }
    }

    /// Create a new transfer action of a single token.
    pub fn single(
        sender_id: impl Into<Cow<'a, AccountIdRef>>,
        receiver_id: impl Into<Cow<'a, AccountIdRef>>,
        token_id: impl Into<Cow<'a, TokenIdRef>>,
        amount: u128,
        memo: Option<impl Into<Cow<'a, str>>>,
    ) -> Self {
        Self {
            receiver_id: receiver_id.into(),
            sender_id: sender_id.into(),
            payload: vec![TokenAmount {
                token_id: token_id.into(),
                amount,
                // approval: None,
            }],
            memo: memo.map(Into::into),
            revert: false,
        }
    }

    /// Adds another token transfer to this action.
    #[must_use]
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
    pub fn memo(mut self, memo: impl Into<Cow<'a, str>>) -> Self {
        self.memo = Some(memo.into());
        self
    }

    /// Add a message string.
    #[must_use]
    pub fn msg(self, msg: impl Into<Cow<'a, str>>) -> Nep245TransferCall<'a> {
        Nep245TransferCall {
            transfer: self,
            msg: msg.into(),
        }
    }

    /// Set revert status.
    #[must_use]
    pub fn revert(self, revert: bool) -> Self {
        Self { revert, ..self }
    }

    /// Returns a vector of the previous owner IDs, as compatible with the
    /// standard arguments of [`Nep245Receiver::mt_on_transfer`].
    #[must_use]
    pub fn previous_owner_ids(&self) -> Vec<AccountId> {
        self.payload
            .iter()
            .map(|_| self.sender_id.clone().into())
            .collect()
    }

    /// Returns a vector of the token IDs, as compatible with the standard
    /// arguments of [`Nep245Receiver::mt_on_transfer`].
    #[must_use]
    pub fn token_ids(&self) -> Vec<TokenId> {
        self.payload
            .iter()
            .map(|token| token.token_id.clone().into())
            .collect()
    }

    /// Returns a vector of the token amounts, as compatible with the standard
    /// arguments of [`Nep245Receiver::mt_on_transfer`].
    #[must_use]
    pub fn amounts(&self) -> Vec<U128> {
        self.payload
            .iter()
            .map(|token| token.amount.into())
            .collect()
    }

    /// Returns a vector of the token approvals, as acompatible with the
    /// standard arguments of [`Nep245Receiver::mt_on_transfer`].
    #[must_use]
    pub fn approvals(&self) -> Option<Vec<Option<MtResolveTransferApproval>>> {
        // TODO: implement real approvals
        Some(vec![None; self.payload.len()])
    }
}

/// A wrapper type for transfers that includes contract invocation metadata.
/// This corresponds to the `*_call` analogues of normal `mt_transfer` functions.
#[derive(PartialEq, Eq, Clone, Debug)]
#[near]
pub struct Nep245TransferCall<'a> {
    transfer: Nep245Transfer<'a>,
    /// Message passed to contract located at `receiver_id`.
    pub msg: Cow<'a, str>,
}

impl<'a> std::ops::Deref for Nep245TransferCall<'a> {
    type Target = Nep245Transfer<'a>;

    fn deref(&self) -> &Self::Target {
        &self.transfer
    }
}

impl<'a> Nep245TransferCall<'a> {
    /// Generates the appropriate [`Promise`] chain for resolving this
    /// transfer-call.
    #[must_use]
    pub fn promise(&self, current_account_id: AccountId) -> Promise {
        let sender_id: AccountId = self.sender_id.clone().into();
        let receiver_id: AccountId = self.receiver_id.clone().into();
        let previous_owner_ids = self.previous_owner_ids();
        let token_ids = self.token_ids();
        let amounts = self.amounts();
        let approvals = self.approvals();
        let msg = self.msg.to_string();
        ext_nep245_receiver::ext(receiver_id.clone())
            .with_unused_gas_weight(10)
            .mt_on_transfer(
                sender_id.clone(),
                previous_owner_ids,
                token_ids.clone(),
                amounts.clone(),
                msg,
            )
            .then(
                ext_nep245_resolver::ext(current_account_id)
                    .with_static_gas(GAS_FOR_MT_RESOLVE_TRANSFER)
                    .with_unused_gas_weight(1)
                    .mt_resolve_transfer(sender_id, receiver_id, token_ids, amounts, approvals),
            )
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

    /// Adds more tokens to this burn action.
    #[must_use]
    pub fn and_burn(mut self, token_id: impl Into<Cow<'a, TokenIdRef>>, amount: u128) -> Self {
        self.payload.push(TokenAmount {
            token_id: token_id.into(),
            amount,
        });
        self
    }

    /// Create a new burn action for burning an amount of a single token.
    #[must_use]
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

    /// Slot for the list of token IDs.
    #[must_use]
    fn slot_tokens() -> Slot<Vector<TokenId>> {
        Self::root().field(StorageKey::Tokens)
    }

    /// Slot for token record.
    #[must_use]
    fn slot_token(token_id: &TokenIdRef) -> Slot<TokenRecord> {
        Self::root().field(StorageKey::Token(token_id))
    }

    /// Slot for account data.
    #[must_use]
    fn slot_balance(token_id: &TokenIdRef, account_id: &AccountIdRef) -> Slot<u128> {
        Self::root().field(StorageKey::Balance(token_id, account_id))
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

    /// Adds a token to the multi token contract.
    ///
    /// # Errors
    ///
    /// - If the token ID is already in use.
    fn create_token(&mut self, token_id: TokenId) -> Result<(), TokenIdCollisionError>;

    /// Get the list of all tokens in this contract.
    fn tokens(&self) -> Vector<TokenId>;

    /// Get the token record for the token ID, should it exist.
    fn token(&self, token_id: &TokenIdRef) -> Option<Token>;

    /// Get the balance of an account. Returns 0 if the account does not exist.
    fn balance_of(&self, token_id: &TokenIdRef, account_id: &AccountIdRef) -> u128;

    /// Get the total circulating supply of the token.
    fn supply(&self, token_id: &TokenIdRef) -> Option<u128>;

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

    /// Performs token transfer reversions (refunds) in the case that tokens
    /// are returned from a transfer-call.
    fn resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        _approvals: Option<Vec<Option<MtResolveTransferApproval>>>,
        mt_on_transfer_result: Option<Vec<U128>>,
    ) -> Vec<U128> {
        let reverts_bounded_by_transfer_value = if let Some(callback_returned) =
            mt_on_transfer_result.filter(|v| v.len() == amounts.len())
        {
            amounts
                .iter()
                .zip(callback_returned)
                .map(|(U128(original_amount), U128(attempted_return))| {
                    u128::min(*original_amount, attempted_return)
                })
                .collect::<Vec<_>>()
        } else {
            amounts.iter().map(|a| a.0).collect()
        };

        // No need to pull in a HashMap for this.
        let mut balance_map = vec![0; token_ids.len()];

        let ix = |token_ids: &[TokenId], token_id: &TokenIdRef| {
            token_ids.iter().position(|i| i == token_id).unwrap()
        };

        for token_id in &token_ids {
            let i = ix(&token_ids, token_id);
            if balance_map[i] == 0 {
                balance_map[i] = self.balance_of(token_id, &receiver_id);
            }
        }

        let mut transfer =
            Nep245Transfer::empty(token_ids.len(), receiver_id, sender_id).revert(true);

        let mut effective_used = Vec::with_capacity(token_ids.len());

        for ((token_id, revert_value), original_amount) in token_ids
            .iter()
            .zip(reverts_bounded_by_transfer_value.iter())
            .zip(amounts.iter())
        {
            let i = ix(&token_ids, token_id);
            let receiver_has = balance_map[i];
            let actual_revert_value = u128::min(*revert_value, receiver_has);
            balance_map[i] = receiver_has - actual_revert_value;
            transfer = transfer.and_transfer(token_id, actual_revert_value);
            effective_used.push(U128(original_amount.0 - actual_revert_value));
        }

        self.transfer(&transfer)
            .unwrap_or_else(|e| env::panic_str(&e.to_string()));

        effective_used
    }
}

impl<T: Nep245ControllerInternal> Nep245Controller for T {
    type MintHook = T::MintHook;
    type TransferHook = T::TransferHook;
    type BurnHook = T::BurnHook;

    fn create_token(&mut self, token_id: TokenId) -> Result<(), TokenIdCollisionError> {
        let mut token_record = Self::slot_token(&token_id);
        if token_record.exists() {
            Err(TokenIdCollisionError { token_id })
        } else {
            token_record.write(&TokenRecord {
                owner_id: None,
                supply: 0,
            });
            let mut tokens = self.tokens();
            tokens.push(&token_id);
            Self::slot_tokens().write(&tokens);
            Ok(())
        }
    }

    fn tokens(&self) -> Vector<TokenId> {
        Self::slot_tokens()
            .read()
            .unwrap_or_else(|| Vector::new(Self::slot_tokens().key))
    }

    fn token(&self, token_id: &TokenIdRef) -> Option<Token> {
        Self::slot_token(token_id).read().map(|token_record| Token {
            token_id: token_id.to_owned(),
            owner_id: token_record.owner_id,
        })
    }

    fn balance_of(&self, token_id: &TokenIdRef, account_id: &AccountIdRef) -> u128 {
        Self::slot_balance(token_id, account_id).read().unwrap_or(0)
    }

    fn supply(&self, token_id: &TokenIdRef) -> Option<u128> {
        Self::slot_token(token_id)
            .read()
            .map(|record| record.supply)
    }

    fn withdraw_unchecked(
        &mut self,
        token_id: &TokenIdRef,
        account_id: &AccountIdRef,
        amount: u128,
    ) -> Result<(), WithdrawError> {
        let mut token_record =
            Self::slot_token(token_id)
                .read()
                .ok_or_else(|| TokenIdDoesNotExistError {
                    token_id: token_id.to_owned(),
                })?;

        if amount != 0 {
            token_record.supply =
                token_record
                    .supply
                    .checked_sub(amount)
                    .ok_or_else(|| SupplyUnderflowError {
                        token_id: token_id.to_owned(),
                        supply: token_record.supply,
                        amount,
                    })?;

            let balance = self.balance_of(token_id, account_id);
            let balance = balance
                .checked_sub(amount)
                .ok_or_else(|| BalanceUnderflowError {
                    token_id: token_id.to_owned(),
                    account_id: account_id.to_owned(),
                    balance,
                    amount,
                })?;

            Self::slot_balance(token_id, account_id).write(&balance);
            Self::slot_token(token_id).write(&token_record);
        }

        Ok(())
    }

    fn deposit_unchecked(
        &mut self,
        token_id: &TokenIdRef,
        account_id: &AccountIdRef,
        amount: u128,
    ) -> Result<(), DepositError> {
        let mut token_record =
            Self::slot_token(token_id)
                .read()
                .ok_or_else(|| TokenIdDoesNotExistError {
                    token_id: token_id.to_owned(),
                })?;

        if amount != 0 {
            let original_supply = token_record.supply;

            token_record.supply =
                token_record
                    .supply
                    .checked_add(amount)
                    .ok_or_else(|| SupplyOverflowError {
                        token_id: token_id.to_owned(),
                        supply: token_record.supply,
                        amount,
                    })?;

            let balance = self.balance_of(token_id, account_id);
            let balance = balance
                .checked_add(amount)
                .ok_or_else(|| BalanceOverflowError {
                    token_id: token_id.to_owned(),
                    account_id: account_id.to_owned(),
                    balance,
                    amount,
                })?;

            if original_supply == 0 && token_record.supply == 1 && token_record.owner_id.is_none() {
                token_record.owner_id = Some(account_id.to_owned());
            }

            Self::slot_balance(token_id, account_id).write(&balance);
            Self::slot_token(token_id).write(&token_record);
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
        if !Self::slot_token(token_id).exists() {
            return Err(TokenIdDoesNotExistError {
                token_id: token_id.to_owned(),
            }
            .into());
        }

        if amount == 0 {
            return Ok(());
        }

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
                if token.amount != 0 {
                    contract.transfer_unchecked(
                        &token.token_id,
                        &transfer.sender_id,
                        &transfer.receiver_id,
                        token.amount,
                    )?;
                    token_ids.push(token.token_id.clone());
                    amounts.push(token.amount.into());
                }
            }

            if !token_ids.is_empty() {
                Nep245Event::MtTransfer(vec![MtTransferData {
                    authorized_id: None,
                    old_owner_id: transfer.sender_id.clone(),
                    new_owner_id: transfer.receiver_id.clone(),
                    token_ids,
                    amounts,
                    memo: transfer.memo.clone(),
                }])
                .emit();
            }

            Ok(())
        })
    }

    fn mint(&mut self, mint: &Nep245Mint) -> Result<(), DepositError> {
        Self::MintHook::hook(self, mint, |contract| {
            let mut token_ids = Vec::with_capacity(mint.payload.len());
            let mut amounts = Vec::with_capacity(mint.payload.len());

            for token in &mint.payload {
                contract.deposit_unchecked(&token.token_id, &mint.receiver_id, token.amount)?;
                token_ids.push(token.token_id.clone());
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
                token_ids.push(token.token_id.clone());
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
