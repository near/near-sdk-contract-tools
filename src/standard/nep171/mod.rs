//! NEP-171 non-fungible token core implementation.
//!
//! Reference: <https://github.com/near/NEPs/blob/master/neps/nep-0171.md>
//!
//! # Usage
//!
//! It is recommended to use the [`near_sdk_contract_tools_macros::Nep171`]
//! derive macro or the [`near_sdk_contract_tools_macros::NonFungibleToken`]
//! macro to implement NEP-171 with this crate.
//!
//! ## Basic implementation with no transfer hooks
//!
//! ```rust
#![doc = include_str!("../../../tests/macros/standard/nep171/no_hooks.rs")]
//! ```
//!
//! ## Basic implementation with transfer hooks
//!
//! ```rust
#![doc = include_str!("../../../tests/macros/standard/nep171/hooks.rs")]
//! ```
//!
//! ## Using the `NonFungibleToken` derive macro for partially-automatic integration with other utilities
//!
//! The `NonFungibleToken` derive macro automatically wires up all of the NFT-related standards' implementations (NEP-171, NEP-177, NEP-178) for you.
//!
//! ```rust
#![doc = include_str!("../../../tests/macros/standard/nep171/non_fungible_token.rs")]
//! ```
//!
//! ## Manual integration with other utilities
//!
//! Note: NFT-related utilities are automatically integrated with each other
//! when using the [`near_sdk_contract_tools_macros::NonFungibleToken`] derive
//! macro.
//! ```rust
#![doc = include_str!("../../../tests/macros/standard/nep171/manual_integration.rs")]
//! ```

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
    type Hook: Nep171Hook<Self>
    where
        Self: Sized;

    /// Invoked during an external transfer.
    type CheckExternalTransfer: CheckExternalTransfer<Self>
    where
        Self: Sized;

    /// Load additional token data into [`Token::extensions_metadata`].
    type LoadTokenMetadata: LoadTokenMetadata<Self>
    where
        Self: Sized;

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
    type Hook: Nep171Hook<Self>
    where
        Self: Sized;

    /// Invoked during an external transfer.
    type CheckExternalTransfer: CheckExternalTransfer<Self>
    where
        Self: Sized;

    /// Load additional token data into [`Token::extensions_metadata`].
    type LoadTokenMetadata: LoadTokenMetadata<Self>
    where
        Self: Sized;

    /// Transfer a token from `sender_id` to `receiver_id`. Checks that the transfer is valid using [`CheckExternalTransfer::check_external_transfer`] before performing the transfer.
    fn external_transfer(&mut self, transfer: &Nep171Transfer) -> Result<(), Nep171TransferError>
    where
        Self: Sized;

    /// Performs a token transfer without running [`CheckExternalTransfer::check_external_transfer`].
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

    fn mint_unchecked(
        &mut self,
        token_ids: &[TokenId],
        new_owner_id: &AccountId,
        memo: Option<String>,
    );

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
    fn load_token(&self, token_id: &TokenId) -> Option<Token>;
}

/// Transfer metadata generic over both types of transfer (`nft_transfer` and
/// `nft_transfer_call`).
#[derive(Serialize, BorshSerialize, PartialEq, Eq, Clone, Debug, Hash)]
pub struct Nep171Transfer<'a> {
    /// Why is this sender allowed to perform this transfer?
    pub authorization: Nep171TransferAuthorization,
    /// Sending account ID. `None` when minting.
    pub sender_id: Option<&'a AccountId>,
    /// Receiving account ID.
    pub receiver_id: &'a AccountId,
    /// Token ID.
    pub token_id: &'a TokenId,
    /// Optional memo string.
    pub memo: Option<&'a str>,
    /// Message passed to contract located at `receiver_id` in the case of `nft_transfer_call`.
    pub msg: Option<&'a str>,
    /// `true` if the transfer is a revert for a `nft_transfer_call`.
    pub revert: bool,
}

/// Authorization for a transfer.
#[derive(Serialize, BorshSerialize, PartialEq, Eq, Clone, Debug, Hash)]
pub enum Nep171TransferAuthorization {
    /// The sender is the owner of the token.
    Owner,
    /// The sender holds a valid approval ID for the token.
    ApprovalId(u32),
}

/// Different ways of checking if a transfer is valid.
pub trait CheckExternalTransfer<C> {
    /// Checks if a transfer is valid. Returns the account ID of the current owner of the token.
    fn check_external_transfer(
        contract: &C,
        transfer: &Nep171Transfer,
    ) -> Result<AccountId, Nep171TransferError>;
}

/// Default external transfer checker. Only allows transfers by the owner of a
/// token. Does not support approval IDs.
pub struct DefaultCheckExternalTransfer;

impl<T: Nep171Controller> CheckExternalTransfer<T> for DefaultCheckExternalTransfer {
    fn check_external_transfer(
        contract: &T,
        transfer: &Nep171Transfer,
    ) -> Result<AccountId, Nep171TransferError> {
        let owner_id = contract.token_owner(transfer.token_id).ok_or_else(|| {
            error::TokenDoesNotExistError {
                token_id: transfer.token_id.clone(),
            }
        })?;

        if let Some(sender_id) = transfer.sender_id {
            // authorizations are only relevent when not minting
            match transfer.authorization {
                Nep171TransferAuthorization::Owner => {
                    if transfer.sender_id != Some(&owner_id) {
                        return Err(error::TokenNotOwnedByExpectedOwnerError {
                            expected_owner_id: sender_id.clone(),
                            owner_id,
                            token_id: transfer.token_id.clone(),
                        }
                        .into());
                    }
                }
                Nep171TransferAuthorization::ApprovalId(approval_id) => {
                    return Err(error::SenderNotApprovedError {
                        owner_id,
                        sender_id: sender_id.clone(),
                        token_id: transfer.token_id.clone(),
                        approval_id,
                    }
                    .into())
                }
            }
        }

        if transfer.receiver_id == &owner_id {
            return Err(error::TokenReceiverIsCurrentOwnerError {
                owner_id,
                token_id: transfer.token_id.clone(),
            }
            .into());
        }

        Ok(owner_id)
    }
}

/// Contracts may implement this trait to inject code into NEP-171 functions.
///
/// `T` is an optional value for passing state between different lifecycle
/// hooks. This may be useful for charging callers for storage usage, for
/// example.
pub trait Nep171Hook<C = Self> {
    type NftTransferState;

    /// Executed before a token transfer is conducted.
    ///
    /// May return an optional state value which will be passed along to the
    /// following `after_transfer`.
    ///
    /// MUST NOT PANIC if the transfer is a revert.
    fn before_nft_transfer(contract: &C, transfer: &Nep171Transfer) -> Self::NftTransferState;

    /// Executed after a token transfer is conducted.
    ///
    /// Receives the state value returned by `before_transfer`.
    ///
    /// MUST NOT PANIC if the transfer is a revert.
    fn after_nft_transfer(
        contract: &mut C,
        transfer: &Nep171Transfer,
        state: Self::NftTransferState,
    );

    //     fn before_mint(contract: &C,
    //         token_ids: &[TokenId],
    //         new_owner_id: &AccountId,
    //         memo: Option<String>,
    // ) {}
}

impl<C> Nep171Hook<C> for () {
    type NftTransferState = ();

    fn before_nft_transfer(_contract: &C, _transfer: &Nep171Transfer) {}

    fn after_nft_transfer(_contract: &mut C, _transfer: &Nep171Transfer, _state: ()) {}
}

impl<Cont, Handl0, Handl1> Nep171Hook<Cont> for (Handl0, Handl1)
where
    Handl0: Nep171Hook<Cont>,
    Handl1: Nep171Hook<Cont>,
{
    type NftTransferState = (Handl0::NftTransferState, Handl1::NftTransferState);

    fn before_nft_transfer(
        contract: &Cont,
        transfer: &Nep171Transfer,
    ) -> (Handl0::NftTransferState, Handl1::NftTransferState) {
        (
            Handl0::before_nft_transfer(contract, transfer),
            Handl1::before_nft_transfer(contract, transfer),
        )
    }

    fn after_nft_transfer(
        contract: &mut Cont,
        transfer: &Nep171Transfer,
        state: (Handl0::NftTransferState, Handl1::NftTransferState),
    ) {
        Handl0::after_nft_transfer(contract, transfer, state.0);
        Handl1::after_nft_transfer(contract, transfer, state.1);
    }
}

impl<T: Nep171ControllerInternal> Nep171Controller for T {
    type Hook = <Self as Nep171ControllerInternal>::Hook;
    type CheckExternalTransfer = <Self as Nep171ControllerInternal>::CheckExternalTransfer;
    type LoadTokenMetadata = <Self as Nep171ControllerInternal>::LoadTokenMetadata;

    fn external_transfer(&mut self, transfer: &Nep171Transfer) -> Result<(), Nep171TransferError> {
        match Self::CheckExternalTransfer::check_external_transfer(self, transfer) {
            Ok(current_owner_id) => {
                let state = <Self as Nep171Controller>::Hook::before_nft_transfer(self, transfer);

                if let Some(sender_id) = transfer.sender_id {
                    // transfer
                    self.transfer_unchecked(
                        &[transfer.token_id.to_string()],
                        current_owner_id,
                        sender_id.clone(),
                        transfer.receiver_id.clone(),
                        transfer.memo.map(ToString::to_string),
                    );
                } else {
                    // mint
                    self.mint_unchecked(
                        &[transfer.token_id.to_string()],
                        transfer.receiver_id,
                        transfer.memo.map(ToString::to_string),
                    )
                }

                <Self as Nep171Controller>::Hook::after_nft_transfer(self, transfer, state);

                Ok(())
            }
            Err(e) => Err(e),
        }
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

    fn mint_unchecked(
        &mut self,
        token_ids: &[TokenId],
        new_owner_id: &AccountId,
        memo: Option<String>,
    ) {
        if token_ids.is_empty() {
            return;
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
    }

    fn mint(
        &mut self,
        token_ids: &[TokenId],
        new_owner_id: &AccountId,
        memo: Option<String>,
    ) -> Result<(), Nep171MintError> {
        for token_id in token_ids {
            let slot = Self::slot_token_owner(token_id);
            if slot.exists() {
                return Err(error::TokenAlreadyExistsError {
                    token_id: token_id.to_string(),
                }
                .into());
            }
        }

        self.mint_unchecked(token_ids, new_owner_id, memo);

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
                        owner_id: actual_owner_id,
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

    fn load_token(&self, token_id: &TokenId) -> Option<Token> {
        let mut metadata = std::collections::HashMap::new();
        Self::LoadTokenMetadata::load(self, token_id, &mut metadata).ok()?;
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
