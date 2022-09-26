use near_sdk::{
    borsh::{self, BorshSerialize},
    ext_contract, AccountId, BorshStorageKey, PromiseOrValue,
};
use serde::Serialize;

use crate::{near_contract_tools, slot::Slot, Event};

pub type TokenId = String;

#[derive(Serialize, Event)]
#[event(standard = "nep171", version = "1.0.0", rename_all = "snake_case")]
#[serde(untagged)]
pub enum Nep171Event<'a> {
    NftMint {
        owner_id: &'a AccountId,
        token_ids: &'a [&'a str],
        #[serde(skip_serializing_if = "Option::is_none")]
        memo: Option<&'a str>,
    },
    NftTransfer {
        old_owner_id: &'a AccountId,
        new_owner_id: &'a AccountId,
        token_ids: &'a [&'a str],
        #[serde(skip_serializing_if = "Option::is_none")]
        authorized_id: Option<&'a AccountId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        memo: Option<&'a str>,
    },
    NftBurn {
        owner_id: &'a AccountId,
        token_ids: &'a [&'a str],
        #[serde(skip_serializing_if = "Option::is_none")]
        authorized_id: Option<&'a AccountId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        memo: Option<&'a str>,
    },
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    TokenOwner(TokenId),
}

pub trait Nep171Controller {
    fn root() -> Slot<()>;

    fn slot_token_owner(token_id: TokenId) -> Slot<AccountId> {
        Self::root().field(StorageKey::TokenOwner(token_id))
    }

    fn transfer_unchecked(token_id: TokenId, receiver_id: &AccountId) -> Option<AccountId> {
        let mut slot = Self::slot_token_owner(token_id);
        if slot.exists() {
            slot.swap(receiver_id)
        } else {
            None
        }
    }

    fn transfer(token_id: TokenId, sender_id: &AccountId, receiver_id: &AccountId) {}

    fn mint_unchecked(token_id: TokenId, new_owner_id: &AccountId) -> bool {
        let mut slot = Self::slot_token_owner(token_id);
        if !slot.exists() {
            slot.write(new_owner_id);
            true
        } else {
            false
        }
    }

    fn burn_unchecked(token_id: TokenId) -> bool {
        Self::slot_token_owner(token_id).remove()
    }
}

pub trait Token {
    fn get_for(&self, token_id: TokenId, owner_id: AccountId) -> Self;
}

#[ext_contract(ext_nep171)]
pub trait Nep171External<Tok> {
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

    fn nft_token(&self, token_id: TokenId) -> Option<Tok>;
}
