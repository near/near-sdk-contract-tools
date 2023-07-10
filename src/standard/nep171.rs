use std::collections::HashMap;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    ext_contract, AccountId, BorshStorageKey, Gas, PromiseOrValue,
};
use near_sdk_contract_tools_macros::event;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{slot::Slot, DefaultStorageKey};

use super::nep297::Event;

pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(5_000_000_000_000);
pub const GAS_FOR_NFT_TRANSFER_CALL: Gas = Gas(25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.0);
pub const INSUFFICIENT_GAS_MESSAGE: &str = "More gas is required";
pub const APPROVAL_MANAGEMENT_NOT_SUPPORTED_MESSAGE: &str =
    "NEP-178: Approval Management is not supported";

pub type TokenId = String;

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

    use super::TokenId;

    #[derive(Serialize, Debug, Clone)]
    pub struct NftMintLog {
        pub owner_id: AccountId,
        pub token_ids: Vec<TokenId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    #[derive(Serialize, Debug, Clone)]
    pub struct NftTransferLog {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub authorized_id: Option<AccountId>,
        pub old_owner_id: AccountId,
        pub new_owner_id: AccountId,
        pub token_ids: Vec<TokenId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    #[derive(Serialize, Debug, Clone)]
    pub struct NftBurnLog {
        pub owner_id: AccountId,
        pub token_ids: Vec<TokenId>,
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

pub mod error {
    use near_sdk::AccountId;
    use thiserror::Error;

    use super::TokenId;

    #[derive(Error, Clone, Debug)]
    #[error("Token `{token_id}` already exists")]
    pub struct AlreadyExists {
        pub token_id: TokenId,
    }

    #[derive(Error, Clone, Debug)]
    #[error("Token `{token_id}` does not exist")]
    pub struct DoesNotExist {
        pub token_id: TokenId,
    }

    #[derive(Error, Clone, Debug)]
    #[error(
        "Token `{token_id}` is owned by `{actual_owner_id}` instead of expected `{expected_owner_id}`",
    )]
    pub struct NotOwnedByExpectedOwner {
        pub expected_owner_id: AccountId,
        pub actual_owner_id: AccountId,
        pub token_id: TokenId,
    }

    #[derive(Error, Clone, Debug)]
    #[error("Sender `{sender_id}` does not have permission to transfer token `{token_id}`")]
    pub struct NotApproved {
        pub sender_id: AccountId,
        pub token_id: TokenId,
    }

    #[derive(Error, Clone, Debug)]
    #[error("Receiver must be different from current owner `{current_owner_id}` to transfer token `{token_id}`")]
    pub struct ReceiverIsCurrentOwner {
        pub current_owner_id: AccountId,
        pub token_id: TokenId,
    }
}

#[derive(Error, Clone, Debug)]
pub enum Nep171BurnError {
    #[error(transparent)]
    DoesNotExist(#[from] error::DoesNotExist),
    #[error(transparent)]
    NotOwnedByExpectedOwner(#[from] error::NotOwnedByExpectedOwner),
}

#[derive(Error, Clone, Debug)]
pub enum Nep171MintError {
    #[error(transparent)]
    AlreadyExists(#[from] error::AlreadyExists),
}

#[derive(Error, Clone, Debug)]
pub enum Nep171TransferError {
    #[error(transparent)]
    DoesNotExist(#[from] error::DoesNotExist),
    #[error(transparent)]
    NotApproved(#[from] error::NotApproved),
    #[error(transparent)]
    ReceiverIsCurrentOwner(#[from] error::ReceiverIsCurrentOwner),
    #[error(transparent)]
    NotOwnedByExpectedOwner(#[from] error::NotOwnedByExpectedOwner),
}
pub trait Nep171Extension<T> {
    type Event: crate::standard::nep297::Event;

    fn handle_transfer(
        result: Result<Nep171Event, Nep171TransferError>,
    ) -> Result<Self::Event, Nep171TransferError>;
}

pub trait Nep171ControllerInternal {
    fn root() -> Slot<()> {
        Slot::root(DefaultStorageKey::Nep171)
    }

    fn slot_token_owner(token_id: TokenId) -> Slot<AccountId> {
        Self::root().field(StorageKey::TokenOwner(token_id))
    }
}

pub trait Nep171Controller {
    fn transfer(
        &mut self,
        token_id: TokenId,
        current_owner_id: AccountId,
        sender_id: AccountId,
        receiver_id: AccountId,
        memo: Option<String>,
    ) -> Result<(), Nep171TransferError>;

    fn mint(&mut self, token_id: TokenId, new_owner_id: &AccountId) -> Result<(), Nep171MintError>;

    fn burn(
        &mut self,
        token_id: TokenId,
        current_owner_id: &AccountId,
    ) -> Result<(), Nep171BurnError>;

    fn burn_unchecked(&mut self, token_id: TokenId) -> bool;

    fn token_owner(&self, token_id: TokenId) -> Option<AccountId>;
}

/// Transfer metadata generic over both types of transfer (`nft_transfer` and
/// `nft_transfer_call`).
#[derive(
    Serialize, Deserialize, BorshSerialize, BorshDeserialize, PartialEq, Eq, Clone, Debug, Hash,
)]
pub struct Nep171Transfer {
    /// Current owner account ID.
    pub owner_id: AccountId,
    /// Sending account ID.
    pub sender_id: AccountId,
    /// Receiving account ID.
    pub receiver_id: AccountId,
    /// Optional approval ID.
    pub approval_id: Option<u64>,
    /// Token ID.
    pub token_id: TokenId,
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
    fn before_nft_transfer(&mut self, _transfer: &Nep171Transfer) -> T;

    /// Executed after a token transfer is conducted.
    ///
    /// Receives the state value returned by `before_transfer`.
    fn after_nft_transfer(&mut self, _transfer: &Nep171Transfer, _state: T);
}

impl<T: Nep171ControllerInternal> Nep171Controller for T {
    fn transfer(
        &mut self,
        token_id: TokenId,
        current_owner_id: AccountId,
        sender_id: AccountId,
        receiver_id: AccountId,
        memo: Option<String>,
    ) -> Result<(), Nep171TransferError> {
        if current_owner_id == receiver_id {
            return Err(error::ReceiverIsCurrentOwner {
                current_owner_id,
                token_id,
            }
            .into());
        }

        // This version doesn't implement approval management
        if sender_id != current_owner_id {
            return Err(error::NotApproved {
                sender_id,
                token_id,
            }
            .into());
        }

        let mut slot = Self::slot_token_owner(token_id.clone());

        let actual_current_owner_id = if let Some(owner_id) = slot.read() {
            owner_id
        } else {
            // Using if-let instead of .ok_or_else() to avoid .clone()
            return Err(error::DoesNotExist { token_id }.into());
        };

        if current_owner_id != actual_current_owner_id {
            return Err(error::NotOwnedByExpectedOwner {
                expected_owner_id: current_owner_id,
                actual_owner_id: actual_current_owner_id,
                token_id,
            }
            .into());
        }

        slot.write(&receiver_id);

        Nep171Event::NftTransfer(vec![event::NftTransferLog {
            authorized_id: None,
            old_owner_id: actual_current_owner_id,
            new_owner_id: receiver_id,
            token_ids: vec![token_id],
            memo,
        }])
        .emit();

        Ok(())
    }

    fn mint(&mut self, token_id: TokenId, new_owner_id: &AccountId) -> Result<(), Nep171MintError> {
        let mut slot = Self::slot_token_owner(token_id.clone());
        if !slot.exists() {
            slot.write(new_owner_id);
            Ok(())
        } else {
            Err(error::AlreadyExists { token_id }.into())
        }
    }

    fn burn(
        &mut self,
        token_id: TokenId,
        current_owner_id: &AccountId,
    ) -> Result<(), Nep171BurnError> {
        let mut slot = Self::slot_token_owner(token_id.clone());
        let actual_owner_id = if let Some(account_id) = slot.read() {
            account_id
        } else {
            return Err(error::DoesNotExist { token_id }.into());
        };

        if current_owner_id != &actual_owner_id {
            return Err(error::NotOwnedByExpectedOwner {
                expected_owner_id: current_owner_id.clone(),
                actual_owner_id,
                token_id,
            }
            .into());
        }

        slot.remove();
        Ok(())
    }

    fn burn_unchecked(&mut self, token_id: TokenId) -> bool {
        Self::slot_token_owner(token_id).remove()
    }

    fn token_owner(&self, token_id: TokenId) -> Option<AccountId> {
        Self::slot_token_owner(token_id).read()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub token_id: TokenId,
    pub owner_id: AccountId,
}

// separate module with re-export because ext_contract doesn't play well with #![warn(missing_docs)]
mod ext {
    #![allow(missing_docs)]

    use std::collections::HashMap;

    use near_sdk::{ext_contract, AccountId, PromiseOrValue};

    use super::{Token, TokenId};

    #[ext_contract(ext_nep171)]
    pub trait Nep171 {
        fn nft_transfer(
            &mut self,
            receiver_id: AccountId,
            token_id: TokenId,
            approval_id: Option<u64>,
            memo: Option<String>,
        );

        fn nft_transfer_call(
            &mut self,
            receiver_id: AccountId,
            token_id: TokenId,
            approval_id: Option<u64>,
            memo: Option<String>,
            msg: String,
        ) -> PromiseOrValue<bool>;

        fn nft_token(&self, token_id: TokenId) -> Option<Token>;
    }

    #[ext_contract(ext_nep171_resolver)]
    pub trait Nep171Resolver {
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
        /// Returns the number of tokens "used", that is, those that will be kept
        /// in the receiving contract's account. (The contract will attempt to
        /// refund the difference from `amount` to the original sender.)
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
