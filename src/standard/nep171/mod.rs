//! NEP-171 non-fungible token core implementation.
//!
//! Reference: <https://github.com/near/NEPs/blob/master/neps/nep-0171.md>

use std::error::Error;

use near_sdk::{
    borsh::{self, BorshSerialize},
    serde::{Deserialize, Serialize},
    AccountId, BorshStorageKey, Gas,
};
use near_sdk_contract_tools_macros::event;
use thiserror::Error;

use crate::{slot::Slot, DefaultStorageKey};

use super::nep297::Event;

pub mod error;
pub mod event;
// separate module with re-export because ext_contract doesn't play well with #![warn(missing_docs)]
mod ext;
pub use ext::*;

/// Minimum required gas for [`Nep171Resolver::nft_resolve_transfer`] call in promise chain during [`Nep171::nft_transfer_call`].
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(5_000_000_000_000);
/// Minimum gas required to execute the main body of [`Nep171::nft_transfer_call`] + gas for [`Nep171Resolver::nft_resolve_transfer`].
pub const GAS_FOR_NFT_TRANSFER_CALL: Gas = Gas(25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.0);
/// Error message when insufficient gas is attached to function calls with a minimum attached gas requirement (i.e. those that produce a promise chain, perform cross-contract calls).
pub const INSUFFICIENT_GAS_MESSAGE: &str = "More gas is required";

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

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey<'a> {
    TokenOwner(&'a str),
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
    ///
    /// NOTE: If you only implement NEP-171, approval IDs will _not work_, and this error will always be returned whenever the sender is not the current owner.
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
    fn external_transfer<Check: CheckExternalTransfer<Self>>(
        &mut self,
        transfer: &Nep171Transfer,
    ) -> Result<(), Nep171TransferError>
    where
        Self: Sized;

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

    /// Loads the metadata associated with a token.
    fn load_token<T: LoadTokenMetadata<Self>>(&self, token_id: &TokenId) -> Option<Token>
    where
        Self: Sized;
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
    pub approval_id: Option<u32>,
    /// Token ID.
    pub token_id: &'a TokenId,
    /// Optional memo string.
    pub memo: Option<&'a str>,
    /// Message passed to contract located at `receiver_id` in the case of `nft_transfer_call`.
    pub msg: Option<&'a str>,
}

/// Different ways of checking if a transfer is valid.
pub trait CheckExternalTransfer<C> {
    /// Checks if a transfer is valid.
    fn check_external_transfer(
        contract: &C,
        transfer: &Nep171Transfer,
    ) -> Result<(), Nep171TransferError>;
}

pub struct DefaultCheckExternalTransfer;

impl<T: Nep171Controller> CheckExternalTransfer<T> for DefaultCheckExternalTransfer {
    fn check_external_transfer(
        contract: &T,
        transfer: &Nep171Transfer,
    ) -> Result<(), Nep171TransferError> {
        contract.check_transfer(
            &[transfer.token_id.to_string()],
            transfer.owner_id,
            transfer.sender_id,
            transfer.receiver_id,
        )
    }
}

/// Contracts may implement this trait to inject code into NEP-171 functions.
///
/// `T` is an optional value for passing state between different lifecycle
/// hooks. This may be useful for charging callers for storage usage, for
/// example.
pub trait Nep171Hook<S = (), C = Self> {
    // TODO: Switch order of C, S generics
    /// Executed before a token transfer is conducted.
    ///
    /// May return an optional state value which will be passed along to the
    /// following `after_transfer`.
    ///
    /// MUST NOT PANIC.
    fn before_nft_transfer(contract: &C, transfer: &Nep171Transfer) -> S;

    /// Executed after a token transfer is conducted.
    ///
    /// Receives the state value returned by `before_transfer`.
    ///
    /// MUST NOT PANIC.
    fn after_nft_transfer(contract: &mut C, transfer: &Nep171Transfer, state: S);
}

impl<C> Nep171Hook<(), C> for () {
    fn before_nft_transfer(_contract: &C, _transfer: &Nep171Transfer) {}

    fn after_nft_transfer(_contract: &mut C, _transfer: &Nep171Transfer, _state: ()) {}
}

impl<Cont, Stat0, Stat1, Handl0, Handl1> Nep171Hook<(Stat0, Stat1), Cont> for (Handl0, Handl1)
where
    Handl0: Nep171Hook<Stat0, Cont>,
    Handl1: Nep171Hook<Stat1, Cont>,
{
    fn before_nft_transfer(contract: &Cont, transfer: &Nep171Transfer) -> (Stat0, Stat1) {
        (
            Handl0::before_nft_transfer(contract, transfer),
            Handl1::before_nft_transfer(contract, transfer),
        )
    }

    fn after_nft_transfer(contract: &mut Cont, transfer: &Nep171Transfer, state: (Stat0, Stat1)) {
        Handl0::after_nft_transfer(contract, transfer, state.0);
        Handl1::after_nft_transfer(contract, transfer, state.1);
    }
}

impl<T: Nep171ControllerInternal> Nep171Controller for T {
    fn external_transfer<Check: CheckExternalTransfer<T>>(
        &mut self,
        transfer: &Nep171Transfer,
    ) -> Result<(), Nep171TransferError> {
        match Check::check_external_transfer(self, transfer) {
            Ok(()) => {
                self.transfer_unchecked(
                    &[transfer.token_id.to_string()],
                    transfer.owner_id.clone(),
                    transfer.sender_id.clone(),
                    transfer.receiver_id.clone(),
                    transfer.memo.map(ToString::to_string),
                );
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

    fn load_token<L: LoadTokenMetadata<Self>>(&self, token_id: &TokenId) -> Option<Token> {
        let mut metadata = std::collections::HashMap::new();
        L::load(self, token_id, &mut metadata).ok()?;
        Some(Token {
            token_id: token_id.clone(),
            owner_id: self.token_owner(token_id)?,
            extensions_metadata: metadata,
        })
    }
}

/// Token information structure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    /// Token ID.
    pub token_id: TokenId,
    /// Current owner of the token.
    pub owner_id: AccountId,
    /// Metadata provided by extensions.
    #[serde(flatten)]
    pub extensions_metadata: std::collections::HashMap<String, near_sdk::serde_json::Value>,
}

/// Trait for NFT extensions to load token metadata.
pub trait LoadTokenMetadata<C> {
    /// Load token metadata into `metadata`.
    fn load(
        contract: &C,
        token_id: &TokenId,
        metadata: &mut std::collections::HashMap<String, near_sdk::serde_json::Value>,
    ) -> Result<(), Box<dyn Error>>;
}

impl<C> LoadTokenMetadata<C> for () {
    fn load(
        _contract: &C,
        _token_id: &TokenId,
        _metadata: &mut std::collections::HashMap<String, near_sdk::serde_json::Value>,
    ) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

impl<C, T: LoadTokenMetadata<C>> LoadTokenMetadata<C> for (T,) {
    fn load(
        contract: &C,
        token_id: &TokenId,
        metadata: &mut std::collections::HashMap<String, near_sdk::serde_json::Value>,
    ) -> Result<(), Box<dyn Error>> {
        T::load(contract, token_id, metadata)?;
        Ok(())
    }
}

impl<C, T: LoadTokenMetadata<C>, U: LoadTokenMetadata<C>> LoadTokenMetadata<C> for (T, U) {
    fn load(
        contract: &C,
        token_id: &TokenId,
        metadata: &mut std::collections::HashMap<String, near_sdk::serde_json::Value>,
    ) -> Result<(), Box<dyn Error>> {
        T::load(contract, token_id, metadata)?;
        U::load(contract, token_id, metadata)?;
        Ok(())
    }
}

// further variations are technically unnecessary: just use (T, (U, V)) or ((T, U), V)
