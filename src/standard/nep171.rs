use near_sdk::{
    borsh::{self, BorshSerialize},
    ext_contract, AccountId, BorshStorageKey, PromiseOrValue,
};
use thiserror::Error;

use crate::slot::Slot;

use super::nep297::Event;

pub mod event {
    use near_sdk::AccountId;
    use serde::Serialize;

    use crate::event;

    #[event(
        standard = "nep171",
        version = "1.0.0",
        crate = "crate",
        macros = "near_contract_tools_macros"
    )]
    #[derive(Debug, Clone)]
    pub struct NftMint(pub Vec<NftMintData>);

    #[derive(Serialize, Debug, Clone)]
    pub struct NftMintData {
        pub owner_id: AccountId,
        pub token_ids: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    #[event(
        standard = "nep171",
        version = "1.0.0",
        crate = "crate",
        macros = "near_contract_tools_macros"
    )]
    #[derive(Debug, Clone)]
    pub struct NftTransfer(pub Vec<NftTransferData>);

    #[derive(Serialize, Debug, Clone)]
    pub struct NftTransferData {
        pub old_owner_id: AccountId,
        pub new_owner_id: AccountId,
        pub token_ids: Vec<String>,
        // #[serde(skip_serializing_if = "Option::is_none")]
        // pub authorized_id: Option<&'a AccountId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub memo: Option<String>,
    }

    #[event(
        standard = "nep171",
        version = "1.0.0",
        crate = "crate",
        macros = "near_contract_tools_macros"
    )]
    #[derive(Debug, Clone)]
    pub struct NftBurn(pub Vec<NftBurnData>);

    #[derive(Serialize, Debug, Clone)]
    pub struct NftBurnData {
        pub owner_id: AccountId,
        pub token_ids: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub authorized_id: Option<AccountId>,
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
}

pub trait Nep171Extension<T> {
    type Event: crate::standard::nep297::Event;

    fn handle_transfer(
        result: Result<event::NftTransfer, Nep171TransferError>,
    ) -> Result<Self::Event, Nep171TransferError>;
}

pub trait Nep171Controller {
    fn root() -> Slot<()>;

    fn slot_token_owner(token_id: String) -> Slot<AccountId> {
        Self::root().field(StorageKey::TokenOwner(token_id))
    }

    fn transfer_unchecked(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        receiver_id: AccountId,
        memo: Option<String>,
    ) -> Result<event::NftTransfer, Nep171TransferError> {
        let mut slot = Self::slot_token_owner(token_id.clone());
        if slot.exists()
            && slot
                .read()
                .map(|current_owner_id| sender_id == current_owner_id)
                .unwrap_or(false)
        {
            slot.write(&receiver_id);
            Ok(event::NftTransfer(vec![event::NftTransferData {
                old_owner_id: sender_id,
                new_owner_id: receiver_id,
                token_ids: vec![token_id],
                memo,
            }]))
        } else {
            Err(Nep171TransferError::SenderIsNotOwner)
        }
    }

    fn transfer(
        &mut self,
        token_id: String,
        sender_id: AccountId,
        receiver_id: AccountId,
        memo: Option<String>,
    ) -> Result<(), Nep171TransferError> {
        self.transfer_unchecked(token_id, sender_id, receiver_id, memo)
            .map(|e| {
                e.emit();
            })
    }

    fn mint_unchecked(token_id: String, new_owner_id: &AccountId) -> bool {
        let mut slot = Self::slot_token_owner(token_id);
        if !slot.exists() {
            slot.write(new_owner_id);
            true
        } else {
            false
        }
    }

    fn burn_unchecked(token_id: String) -> bool {
        Self::slot_token_owner(token_id).remove()
    }
}

pub trait Token {
    fn get_for(&self, token_id: String, owner_id: AccountId) -> Self;
}

#[ext_contract(ext_nep171)]
pub trait Nep171External<Tok> {
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

    fn nft_token(&self, token_id: String) -> Option<Tok>;
}
