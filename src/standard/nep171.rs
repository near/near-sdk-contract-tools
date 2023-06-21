use std::collections::HashMap;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    ext_contract, AccountId, BorshStorageKey, Gas, PromiseOrValue,
};
use near_sdk_contract_tools_macros::event;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::slot::Slot;

use super::nep297::Event;

pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(5_000_000_000_000);
pub const GAS_FOR_NFT_TRANSFER_CALL: Gas = Gas(25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.0);
pub const INSUFFICIENT_GAS_MESSAGE: &str = "More gas is required";
pub const APPROVAL_MANAGEMENT_NOT_SUPPORTED_MESSAGE: &str = "NEP-178: Approval Management is not supported";

#[event(
    crate = "crate",
    macros = "crate",
    serde = "serde",
    standard = "nep171",
    version = "1.1.0"
)]
#[derive(Debug, Clone)]
pub enum Nep171Event {
    NftMint(Vec<event::NftMintLog>),
    NftTransfer(Vec<event::NftTransferLog>),
    NftBurn(Vec<event::NftBurnLog>),
    ContractMetadataUpdate(Vec<event::ContractMetadataUpdateLog>),
}

pub mod event {
    use near_sdk::AccountId;
    use serde::Serialize;

    #[derive(Serialize, Debug, Clone)]
    pub struct NftMintLog {
        pub owner_id: AccountId,
        pub token_ids: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    #[derive(Serialize, Debug, Clone)]
    pub struct NftTransferLog {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub authorized_id: Option<AccountId>,
        pub old_owner_id: AccountId,
        pub new_owner_id: AccountId,
        pub token_ids: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    #[derive(Serialize, Debug, Clone)]
    pub struct NftBurnLog {
        pub owner_id: AccountId,
        pub token_ids: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub authorized_id: Option<AccountId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    #[derive(Serialize, Debug, Clone)]
    pub struct ContractMetadataUpdateLog {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    TokenOwner(String),
}

#[derive(Error, Clone, Debug)]
pub enum Nep171TransferError {
    #[error("Sender is not the owner")]
    SenderIsNotOwner,
    #[error("Sender and receiver must be different")]
    SenderEqualsReceiver,
}

pub trait Nep171Extension<T> {
    type Event: crate::standard::nep297::Event;

    fn handle_transfer(
        result: Result<Nep171Event, Nep171TransferError>,
    ) -> Result<Self::Event, Nep171TransferError>;
}

pub trait Nep171ControllerInternal {
    fn root() -> Slot<()>;

    fn slot_token_owner(token_id: String) -> Slot<AccountId> {
        Self::root().field(StorageKey::TokenOwner(token_id))
    }
}

pub trait Nep171Controller {
    fn transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        receiver_id: AccountId,
        memo: Option<String>,
    ) -> Result<(), Nep171TransferError>;

    fn mint(token_id: String, new_owner_id: &AccountId) -> bool;

    fn burn(token_id: String) -> bool;

    fn token_owner(&self, token_id: String) -> Option<AccountId>;
}

/// Transfer metadata generic over both types of transfer (`nft_transfer` and
/// `nft_transfer_call`).
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone, Debug)]
pub struct Nep171Transfer {
    /// Sending account ID.
    pub sender_id: AccountId,
    /// Receiving account ID.
    pub receiver_id: AccountId,
    /// Optional approval ID.
    pub approval_id: Option<u64>,
    /// Token ID.
    pub token_id: String,
    /// Optional memo string.
    pub memo: Option<String>,
    /// Message passed to contract located at `receiver_id` in the case of `nft_transfer_call`.
    pub msg: Option<String>,
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
    fn before_transfer(&mut self, _transfer: &Nep171Transfer) -> T;

    /// Executed after a token transfer is conducted.
    ///
    /// Receives the state value returned by `before_transfer`.
    fn after_transfer(&mut self, _transfer: &Nep171Transfer, _state: T) {}
}

impl<T: Nep171ControllerInternal> Nep171Controller for T {
    fn transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        receiver_id: AccountId,
        memo: Option<String>,
    ) -> Result<(), Nep171TransferError> {
        if sender_id == receiver_id {
            return Err(Nep171TransferError::SenderEqualsReceiver);
        }

        let mut slot = Self::slot_token_owner(token_id.clone());

        if slot.exists()
            && slot
                .read()
                .map(|current_owner_id| sender_id == current_owner_id)
                .unwrap_or(false)
        {
            slot.write(&receiver_id);

            Nep171Event::NftTransfer(vec![event::NftTransferLog {
                authorized_id: None,
                old_owner_id: sender_id,
                new_owner_id: receiver_id,
                token_ids: vec![token_id],
                memo,
            }])
            .emit();

            Ok(())
        } else {
            Err(Nep171TransferError::SenderIsNotOwner)
        }
    }

    fn mint(token_id: String, new_owner_id: &AccountId) -> bool {
        let mut slot = Self::slot_token_owner(token_id);
        if !slot.exists() {
            slot.write(new_owner_id);
            true
        } else {
            false
        }
    }

    fn burn(token_id: String) -> bool {
        Self::slot_token_owner(token_id).remove()
    }

    fn token_owner(&self, token_id: String) -> Option<AccountId> {
        Self::slot_token_owner(token_id).read()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub token_id: String,
    pub owner_id: AccountId,
}

#[ext_contract(ext_nep171)]
pub trait Nep171 {
    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        approval_id: Option<u64>,
        memo: Option<String>,
    );

    fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: String,
        approval_id: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<bool>;

    fn nft_token(&self, token_id: String) -> Option<Token>;

    fn nft_resolve_transfer(
        previous_owner_id: AccountId,
        receiver_id: AccountId,
        token_id: String,
        approved_account_ids: Option<HashMap<String, u64>>,
    ) -> bool;
}

/// A contract that may be the recipient of an `nft_transfer_call` function
/// call.
#[ext_contract(ext_nep171_receiver)]
pub trait Nep171Receiver {
    /// Function that is called in an `nft_transfer_call` promise chain.
    /// Returns the number of tokens "used", that is, those that will be kept
    /// in the receiving contract's account. (The contract will attempt to
    /// refund the difference from `amount` to the original sender.)
    fn nft_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_id: AccountId,
        token_id: String,
        msg: String,
    ) -> PromiseOrValue<bool>;
}
