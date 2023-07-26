//! NEP-171 non-fungible token core implementation.
//!
//! Reference: <https://github.com/near/NEPs/blob/master/neps/nep-0171.md>

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    AccountId, BorshStorageKey, Gas,
};
use near_sdk_contract_tools_macros::event;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{slot::Slot, DefaultStorageKey};

use super::nep297::Event;

/// Minimum required gas for [`Nep171Resolver::nft_resolve_transfer`] call in promise chain during [`Nep171::nft_transfer_call`].
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(5_000_000_000_000);
/// Minimum gas required to execute the main body of [`Nep171::nft_transfer_call`] + gas for [`Nep171Resolver::nft_resolve_transfer`].
pub const GAS_FOR_NFT_TRANSFER_CALL: Gas = Gas(25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.0);
/// Error message when insufficient gas is attached to function calls with a minimum attached gas requirement (i.e. those that produce a promise chain, perform cross-contract calls).
pub const INSUFFICIENT_GAS_MESSAGE: &str = "More gas is required";
/// Error message when the NEP-171 implementation does not also implement NEP-178.
pub const APPROVAL_MANAGEMENT_NOT_SUPPORTED_MESSAGE: &str =
    "NEP-178: Approval Management is not supported";

/// NFT token IDs.
pub type TokenId = String;

/// NEP-171 standard events.
#[event(
    crate = "crate",
    macros = "crate",
    serde = "serde",
    standard = "nep171",
    version = "1.2.0"
)]
#[derive(Debug, Clone)]
pub enum Nep171Event {
    /// Emitted when a token is newly minted.
    NftMint(Vec<event::NftMintLog>),
    /// Emitted when a token is transferred between two parties.
    NftTransfer(Vec<event::NftTransferLog>),
    /// Emitted when a token is burned.
    NftBurn(Vec<event::NftBurnLog>),
    /// Emitted when the metadata associated with an NFT contract is updated.
    NftMetadataUpdate(Vec<event::NftMetadataUpdateLog>),
    /// Emitted when the metadata associated with an NFT contract is updated.
    ContractMetadataUpdate(Vec<event::NftContractMetadataUpdateLog>),
}

/// Event log metadata & associated structures.
pub mod event {
    use near_sdk::AccountId;
    use serde::Serialize;

    /// Tokens minted to a single owner.
    #[derive(Serialize, Debug, Clone)]
    pub struct NftMintLog {
        /// To whom were the new tokens minted?
        pub owner_id: AccountId,
        /// Which tokens were minted?
        pub token_ids: Vec<String>,
        /// Additional mint information.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    /// Tokens are transferred from one account to another.
    #[derive(Serialize, Debug, Clone)]
    pub struct NftTransferLog {
        /// NEP-178 authorized account ID.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub authorized_id: Option<AccountId>,
        /// Account ID of the previous owner.
        pub old_owner_id: AccountId,
        /// Account ID of the new owner.
        pub new_owner_id: AccountId,
        /// IDs of the transferred tokens.
        pub token_ids: Vec<String>,
        /// Additional transfer information.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    /// Tokens are burned from a single holder.
    #[derive(Serialize, Debug, Clone)]
    pub struct NftBurnLog {
        /// What is the ID of the account from which the tokens were burned?
        pub owner_id: AccountId,
        /// IDs of the burned tokens.
        pub token_ids: Vec<String>,
        /// NEP-178 authorized account ID.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub authorized_id: Option<AccountId>,
        /// Additional burn information.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    /// Token metadata update.
    #[derive(Serialize, Debug, Clone)]
    pub struct NftMetadataUpdateLog {
        /// IDs of the updated tokens.
        pub token_ids: Vec<String>,
        /// Additional update information.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    /// Contract metadata update.
    #[derive(Serialize, Debug, Clone)]
    pub struct NftContractMetadataUpdateLog {
        /// Additional update information.
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey<'a> {
    TokenOwner(&'a str),
}

/// Potential errors produced by various token manipulations.
pub mod error {
    use near_sdk::AccountId;
    use thiserror::Error;

    use super::TokenId;

    /// Occurs when trying to create a token ID that already exists.
    /// Overwriting pre-existing token IDs is not allowed.
    #[derive(Error, Clone, Debug)]
    #[error("Token `{token_id}` already exists")]
    pub struct TokenAlreadyExistsError {
        /// The conflicting token ID.
        pub token_id: TokenId,
    }

    /// When attempting to interact with a non-existent token ID.
    #[derive(Error, Clone, Debug)]
    #[error("Token `{token_id}` does not exist")]
    pub struct TokenDoesNotExistError {
        /// The invalid token ID.
        pub token_id: TokenId,
    }

    /// Occurs when performing a checked operation that expects a token to be
    /// owned by a particular account, but the token is _not_ owned by that
    /// account.
    #[derive(Error, Clone, Debug)]
    #[error(
        "Token `{token_id}` is owned by `{actual_owner_id}` instead of expected `{expected_owner_id}`",
    )]
    pub struct TokenNotOwnedByExpectedOwnerError {
        /// The token was supposed to be owned by this account.
        pub expected_owner_id: AccountId,
        /// The token is actually owned by this account.
        pub actual_owner_id: AccountId,
        /// The ID of the token in question.
        pub token_id: TokenId,
    }

    /// Occurs when a particular account is not allowed to transfer a token (e.g. on behalf of another user). See: NEP-178.
    #[derive(Error, Clone, Debug)]
    #[error("Sender `{sender_id}` does not have permission to transfer token `{token_id}`")]
    pub struct SenderNotApprovedError {
        /// The unapproved sender.
        pub sender_id: AccountId,
        /// The ID of the token in question.
        pub token_id: TokenId,
    }

    /// Occurs when attempting to perform a transfer of a token from one
    /// account to the same account.
    #[derive(Error, Clone, Debug)]
    #[error("Receiver must be different from current owner `{current_owner_id}` to transfer token `{token_id}`")]
    pub struct TokenReceiverIsCurrentOwnerError {
        /// The account ID of current owner of the token.
        pub current_owner_id: AccountId,
        /// The ID of the token in question.
        pub token_id: TokenId,
    }
}

/// Potential errors encountered when performing a burn operation.
#[derive(Error, Clone, Debug)]
pub enum Nep171BurnError {
    /// The token could not be burned because it does not exist.
    #[error(transparent)]
    TokenDoesNotExist(#[from] error::TokenDoesNotExistError),
    /// The token could not be burned because it is not owned by the expected owner.
    #[error(transparent)]
    TokenNotOwnedByExpectedOwner(#[from] error::TokenNotOwnedByExpectedOwnerError),
}

/// Potential errors encountered when attempting to mint a new token.
#[derive(Error, Clone, Debug)]
pub enum Nep171MintError {
    /// The token could not be minted because a token with the same ID already exists.
    #[error(transparent)]
    TokenAlreadyExists(#[from] error::TokenAlreadyExistsError),
}

/// Potential errors encountered when performing a token transfer.
#[derive(Error, Clone, Debug)]
pub enum Nep171TransferError {
    /// The token could not be transferred because it does not exist.
    #[error(transparent)]
    TokenDoesNotExist(#[from] error::TokenDoesNotExistError),
    /// The token could not be transferred because the sender is not allowed to perform transfers of this token on behalf of its current owner. See: NEP-178.
    #[error(transparent)]
    SenderNotApproved(#[from] error::SenderNotApprovedError),
    /// The token could not be transferred because the token is being sent to the account that currently owns it. Reflexive transfers are not allowed.
    #[error(transparent)]
    TokenReceiverIsCurrentOwner(#[from] error::TokenReceiverIsCurrentOwnerError),
    /// The token could not be transferred because it is no longer owned by the expected owner.
    #[error(transparent)]
    TokenNotOwnedByExpectedOwner(#[from] error::TokenNotOwnedByExpectedOwnerError),
}

/// Internal (storage location) methods for implementors of [`Nep171Controller`].
pub trait Nep171ControllerInternal {
    /// Root storage slot.
    fn root() -> Slot<()> {
        Slot::root(DefaultStorageKey::Nep171)
    }

    /// Storage slot for the owner of a token.
    fn slot_token_owner(token_id: &TokenId) -> Slot<AccountId> {
        Self::root().field(StorageKey::TokenOwner(token_id))
    }
}

/// Non-public controller interface for NEP-171 implementations.
pub trait Nep171Controller {
    /// Transfer a token from `sender_id` to `receiver_id`. Checks that the transfer is valid using [`Nep171Controller::check_transfer`] before performing the transfer.
    fn transfer(
        &mut self,
        token_ids: &[TokenId],
        current_owner_id: AccountId,
        sender_id: AccountId,
        receiver_id: AccountId,
        memo: Option<String>,
    ) -> Result<(), Nep171TransferError>;

    /// Check if a token transfer is valid without actually performing it.
    fn check_transfer(
        &self,
        token_ids: &[TokenId],
        current_owner_id: &AccountId,
        sender_id: &AccountId,
        receiver_id: &AccountId,
    ) -> Result<(), Nep171TransferError>;

    /// Performs a token transfer without running [`Nep171Controller::check_transfer`].
    ///
    /// # Warning
    ///
    /// This function performs _no checks_. It is up to the caller to ensure that the transfer is valid. Possible unintended effects of invalid transfers include:
    /// - Transferring a token "from" an account that does not own it.
    /// - Creating token IDs that did not previously exist.
    /// - Transferring a token to the account that already owns it.
    fn transfer_unchecked(
        &mut self,
        token_ids: &[TokenId],
        current_owner_id: AccountId,
        sender_id: AccountId,
        receiver_id: AccountId,
        memo: Option<String>,
    );

    /// Mints a new token `token_id` to `owner_id`.
    fn mint(
        &mut self,
        token_ids: &[TokenId],
        new_owner_id: &AccountId,
        memo: Option<String>,
    ) -> Result<(), Nep171MintError>;

    /// Burns tokens `token_ids` owned by `current_owner_id`.
    fn burn(
        &mut self,
        token_ids: &[TokenId],
        current_owner_id: &AccountId,
        memo: Option<String>,
    ) -> Result<(), Nep171BurnError>;

    /// Burns tokens `token_ids` without checking the owners.
    fn burn_unchecked(&mut self, token_ids: &[TokenId]) -> bool;

    /// Returns the owner of a token, if it exists.
    fn token_owner(&self, token_id: &TokenId) -> Option<AccountId>;
}

/// Transfer metadata generic over both types of transfer (`nft_transfer` and
/// `nft_transfer_call`).
#[derive(Serialize, BorshSerialize, PartialEq, Eq, Clone, Debug, Hash)]
pub struct Nep171Transfer<'a> {
    /// Current owner account ID.
    pub owner_id: &'a AccountId,
    /// Sending account ID.
    pub sender_id: &'a AccountId,
    /// Receiving account ID.
    pub receiver_id: &'a AccountId,
    /// Optional approval ID.
    pub approval_id: Option<u64>,
    /// Token ID.
    pub token_id: &'a TokenId,
    /// Optional memo string.
    pub memo: Option<&'a str>,
    /// Message passed to contract located at `receiver_id` in the case of `nft_transfer_call`.
    pub msg: Option<&'a str>,
}

/// Contracts may implement this trait to inject code into NEP-171 functions.
///
/// `T` is an optional value for passing state between different lifecycle
/// hooks. This may be useful for charging callers for storage usage, for
/// example.
pub trait Nep171Hook<T = ()> {
    /// Executed before a token transfer is conducted.
    ///
    /// May return an optional state value which will be passed along to the
    /// following `after_transfer`.
    ///
    /// MUST NOT PANIC.
    fn before_nft_transfer(&self, transfer: &Nep171Transfer) -> T;

    /// Executed after a token transfer is conducted.
    ///
    /// Receives the state value returned by `before_transfer`.
    ///
    /// MUST NOT PANIC.
    fn after_nft_transfer(&mut self, transfer: &Nep171Transfer, state: T);
}

impl<T: Nep171ControllerInternal> Nep171Controller for T {
    fn transfer(
        &mut self,
        token_ids: &[TokenId],
        current_owner_id: AccountId,
        sender_id: AccountId,
        receiver_id: AccountId,
        memo: Option<String>,
    ) -> Result<(), Nep171TransferError> {
        match self.check_transfer(token_ids, &current_owner_id, &sender_id, &receiver_id) {
            Ok(()) => {
                self.transfer_unchecked(token_ids, current_owner_id, sender_id, receiver_id, memo);
                Ok(())
            }
            e => e,
        }
    }

    fn check_transfer(
        &self,
        token_ids: &[TokenId],
        current_owner_id: &AccountId,
        sender_id: &AccountId,
        receiver_id: &AccountId,
    ) -> Result<(), Nep171TransferError> {
        for token_id in token_ids {
            let slot = Self::slot_token_owner(token_id);

            let actual_current_owner_id =
                slot.read().ok_or_else(|| error::TokenDoesNotExistError {
                    token_id: token_id.clone(),
                })?;

            if current_owner_id != &actual_current_owner_id {
                return Err(error::TokenNotOwnedByExpectedOwnerError {
                    expected_owner_id: current_owner_id.clone(),
                    actual_owner_id: actual_current_owner_id,
                    token_id: token_id.clone(),
                }
                .into());
            }

            // This version doesn't implement approval management
            if sender_id != current_owner_id {
                return Err(error::SenderNotApprovedError {
                    sender_id: sender_id.clone(),
                    token_id: token_id.clone(),
                }
                .into());
            }

            if receiver_id == current_owner_id {
                return Err(error::TokenReceiverIsCurrentOwnerError {
                    current_owner_id: current_owner_id.clone(),
                    token_id: token_id.clone(),
                }
                .into());
            }
        }
        Ok(())
    }

    fn transfer_unchecked(
        &mut self,
        token_ids: &[TokenId],
        current_owner_id: AccountId,
        _sender_id: AccountId,
        receiver_id: AccountId,
        memo: Option<String>,
    ) {
        if !token_ids.is_empty() {
            Nep171Event::NftTransfer(vec![event::NftTransferLog {
                authorized_id: None,
                old_owner_id: current_owner_id,
                new_owner_id: receiver_id.clone(),
                token_ids: token_ids.iter().map(ToString::to_string).collect(),
                memo,
            }])
            .emit();
        }

        for token_id in token_ids {
            let mut slot = Self::slot_token_owner(token_id);
            slot.write(&receiver_id);
        }
    }

    fn mint(
        &mut self,
        token_ids: &[TokenId],
        new_owner_id: &AccountId,
        memo: Option<String>,
    ) -> Result<(), Nep171MintError> {
        if token_ids.is_empty() {
            return Ok(());
        }

        for token_id in token_ids {
            let slot = Self::slot_token_owner(token_id);
            if slot.exists() {
                return Err(error::TokenAlreadyExistsError {
                    token_id: token_id.to_string(),
                }
                .into());
            }
        }

        Nep171Event::NftMint(vec![event::NftMintLog {
            token_ids: token_ids.iter().map(ToString::to_string).collect(),
            owner_id: new_owner_id.clone(),
            memo,
        }])
        .emit();

        token_ids.iter().for_each(|token_id| {
            let mut slot = Self::slot_token_owner(token_id);
            slot.write(new_owner_id);
        });

        Ok(())
    }

    fn burn(
        &mut self,
        token_ids: &[TokenId],
        current_owner_id: &AccountId,
        memo: Option<String>,
    ) -> Result<(), Nep171BurnError> {
        if token_ids.is_empty() {
            return Ok(());
        }

        for token_id in token_ids {
            if let Some(actual_owner_id) = self.token_owner(token_id) {
                if &actual_owner_id != current_owner_id {
                    return Err(error::TokenNotOwnedByExpectedOwnerError {
                        expected_owner_id: current_owner_id.clone(),
                        actual_owner_id,
                        token_id: (*token_id).clone(),
                    }
                    .into());
                }
            } else {
                return Err(error::TokenDoesNotExistError {
                    token_id: (*token_id).clone(),
                }
                .into());
            }
        }

        self.burn_unchecked(token_ids);

        Nep171Event::NftBurn(vec![event::NftBurnLog {
            token_ids: token_ids.iter().map(ToString::to_string).collect(),
            owner_id: current_owner_id.clone(),
            authorized_id: None,
            memo,
        }])
        .emit();

        Ok(())
    }

    fn burn_unchecked(&mut self, token_ids: &[TokenId]) -> bool {
        let mut removed_successfully = true;

        for token_id in token_ids {
            removed_successfully &= Self::slot_token_owner(token_id).remove();
        }

        removed_successfully
    }

    fn token_owner(&self, token_id: &TokenId) -> Option<AccountId> {
        Self::slot_token_owner(token_id).read()
    }
}

/// Token information structure.
#[derive(
    Debug,
    Clone,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct Token {
    /// Token ID.
    pub token_id: TokenId,
    /// Current owner of the token.
    pub owner_id: AccountId,
}

// separate module with re-export because ext_contract doesn't play well with #![warn(missing_docs)]
mod ext {
    #![allow(missing_docs)]

    use std::collections::HashMap;

    use near_sdk::{ext_contract, AccountId, PromiseOrValue};

    use super::{Token, TokenId};

    /// Interface of contracts that implement NEP-171.
    #[ext_contract(ext_nep171)]
    pub trait Nep171 {
        /// Transfer a token.
        fn nft_transfer(
            &mut self,
            receiver_id: AccountId,
            token_id: TokenId,
            approval_id: Option<u64>,
            memo: Option<String>,
        );

        /// Transfer a token, and call [`Nep171Receiver::nft_on_transfer`] on the receiving account.
        fn nft_transfer_call(
            &mut self,
            receiver_id: AccountId,
            token_id: TokenId,
            approval_id: Option<u64>,
            memo: Option<String>,
            msg: String,
        ) -> PromiseOrValue<bool>;

        /// Get individual token information.
        fn nft_token(&self, token_id: TokenId) -> Option<Token>;
    }

    /// Original token contract follow-up to [`Nep171::nft_transfer_call`].
    #[ext_contract(ext_nep171_resolver)]
    pub trait Nep171Resolver {
        /// Final method call on the original token contract during an
        /// [`Nep171::nft_transfer_call`] promise chain.
        fn nft_resolve_transfer(
            &mut self,
            previous_owner_id: AccountId,
            receiver_id: AccountId,
            token_id: TokenId,
            approved_account_ids: Option<HashMap<AccountId, u64>>,
        ) -> bool;
    }

    /// A contract that may be the recipient of an `nft_transfer_call` function
    /// call.
    #[ext_contract(ext_nep171_receiver)]
    pub trait Nep171Receiver {
        /// Function that is called in an `nft_transfer_call` promise chain.
        /// Performs some action after receiving a non-fungible token.
        ///
        /// Returns `true` if token should be returned to `sender_id`.
        fn nft_on_transfer(
            &mut self,
            sender_id: AccountId,
            previous_owner_id: AccountId,
            token_id: TokenId,
            msg: String,
        ) -> PromiseOrValue<bool>;
    }
}

pub use ext::*;
