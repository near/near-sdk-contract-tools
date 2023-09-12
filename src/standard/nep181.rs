//! NEP-181 non-fungible token contract metadata implementation.
//!
//! Reference: <https://github.com/near/NEPs/blob/master/neps/nep-0181.md>
use std::borrow::Cow;

use near_sdk::{
    borsh::{self, BorshSerialize},
    env,
    store::UnorderedSet,
    AccountId, BorshStorageKey,
};

use crate::{slot::Slot, standard::nep171::*, DefaultStorageKey};

pub use ext::*;

pub struct TokenEnumeration {}

impl<C: Nep171Controller + Nep181Controller> Nep171Hook<C> for TokenEnumeration {
    type MintState = ();
    type NftTransferState = ();
    type BurnState = ();

    fn before_mint(_contract: &C, _token_id: &TokenId, _owner_id: &AccountId) {}

    fn after_mint(contract: &mut C, token_id: &TokenId, owner_id: &AccountId, _: ()) {
        contract.add_token_to_enumeration(token_id.clone(), owner_id);
    }

    fn before_nft_transfer(_contract: &C, _transfer: &Nep171Transfer) {}

    fn after_nft_transfer(contract: &mut C, transfer: &Nep171Transfer, _: ()) {
        let owner_id = match transfer.authorization {
            Nep171TransferAuthorization::Owner => Cow::Borrowed(transfer.sender_id),
            Nep171TransferAuthorization::ApprovalId(_) => Cow::Owned(contract.token_owner(transfer.token_id).unwrap_or_else(|| {
                env::panic_str(&format!("Inconsistent state: Enumeration reconciliation should only run after a token has been transferred, but token {} does not exist.", transfer.token_id))
            })),
        };

        contract.transfer_token_enumeration(
            transfer.token_id.clone(),
            owner_id.as_ref(),
            transfer.receiver_id,
        );
    }

    fn before_burn(_contract: &C, _token_id: &TokenId, _owner_id: &AccountId) {}

    fn after_burn(contract: &mut C, token_id: &TokenId, owner_id: &AccountId, _: ()) {
        contract.remove_token_from_enumeration(token_id, owner_id);
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey<'a> {
    Tokens,
    OwnerTokens(&'a AccountId),
}

/// Internal functions for [`Nep181Controller`].
pub trait Nep181ControllerInternal {
    /// Storage root.
    fn root() -> Slot<()> {
        Slot::root(DefaultStorageKey::Nep181)
    }

    fn slot_tokens() -> Slot<UnorderedSet<TokenId>> {
        Self::root().field(StorageKey::Tokens)
    }

    fn slot_owner_tokens(owner_id: &AccountId) -> Slot<UnorderedSet<TokenId>> {
        Self::root().field(StorageKey::OwnerTokens(owner_id))
    }
}

/// Functions for managing non-fungible tokens with attached metadata, NEP-181.
pub trait Nep181Controller {
    fn add_token_to_enumeration(&mut self, token_id: TokenId, owner_id: &AccountId);
    fn remove_token_from_enumeration(&mut self, token_id: &TokenId, owner_id: &AccountId);
    fn transfer_token_enumeration(
        &mut self,
        token_id: TokenId,
        from_owner_id: &AccountId,
        to_owner_id: &AccountId,
    );
    fn total_enumerated_tokens(&self) -> u128;
    fn with_tokens<T>(&self, f: impl FnOnce(&UnorderedSet<TokenId>) -> T) -> T;
    fn with_tokens_for_owner<T>(
        &self,
        owner_id: &AccountId,
        f: impl FnOnce(&UnorderedSet<TokenId>) -> T,
    ) -> T;
}

impl<T: Nep181ControllerInternal + Nep171Controller> Nep181Controller for T {
    fn add_token_to_enumeration(&mut self, token_id: TokenId, owner_id: &AccountId) {
        let mut all_tokens_slot = Self::slot_tokens();
        let mut all_tokens = all_tokens_slot
            .read()
            .unwrap_or_else(|| UnorderedSet::new(StorageKey::Tokens));

        all_tokens.insert(token_id.clone());

        all_tokens_slot.write(&all_tokens);

        let mut owner_tokens_slot = Self::slot_owner_tokens(owner_id);
        let mut owner_tokens = owner_tokens_slot
            .read()
            .unwrap_or_else(|| UnorderedSet::new(StorageKey::OwnerTokens(owner_id)));

        owner_tokens.insert(token_id);

        owner_tokens_slot.write(&owner_tokens);
    }

    fn remove_token_from_enumeration(&mut self, token_id: &TokenId, owner_id: &AccountId) {
        let mut all_tokens_slot = Self::slot_tokens();
        if let Some(mut all_tokens) = all_tokens_slot.read() {
            all_tokens.remove(token_id);
            all_tokens_slot.write(&all_tokens);
        }

        let mut owner_tokens_slot = Self::slot_owner_tokens(owner_id);
        if let Some(mut owner_tokens) = owner_tokens_slot.read() {
            owner_tokens.remove(token_id);
            owner_tokens_slot.write(&owner_tokens);
        }
    }

    fn transfer_token_enumeration(
        &mut self,
        token_id: TokenId,
        from_owner_id: &AccountId,
        to_owner_id: &AccountId,
    ) {
        let mut from_owner_tokens_slot = Self::slot_owner_tokens(from_owner_id);
        if let Some(mut from_owner_tokens) = from_owner_tokens_slot.read() {
            from_owner_tokens.remove(&token_id);
            from_owner_tokens_slot.write(&from_owner_tokens);
        }

        let mut to_owner_tokens_slot = Self::slot_owner_tokens(to_owner_id);
        let mut to_owner_tokens = to_owner_tokens_slot
            .read()
            .unwrap_or_else(|| UnorderedSet::new(StorageKey::OwnerTokens(to_owner_id)));

        to_owner_tokens.insert(token_id);

        to_owner_tokens_slot.write(&to_owner_tokens);
    }

    fn total_enumerated_tokens(&self) -> u128 {
        Self::slot_tokens()
            .read()
            .map(|tokens| tokens.len())
            .unwrap_or_default()
            .into()
    }

    fn with_tokens<U>(&self, f: impl FnOnce(&UnorderedSet<TokenId>) -> U) -> U {
        f(&Self::slot_tokens()
            .read()
            .unwrap_or_else(|| UnorderedSet::new(StorageKey::Tokens)))
    }

    fn with_tokens_for_owner<U>(
        &self,
        owner_id: &AccountId,
        f: impl FnOnce(&UnorderedSet<TokenId>) -> U,
    ) -> U {
        f(&Self::slot_owner_tokens(owner_id)
            .read()
            .unwrap_or_else(|| UnorderedSet::new(StorageKey::OwnerTokens(owner_id))))
    }
}

// separate module with re-export because ext_contract doesn't play well with #![warn(missing_docs)]
mod ext {
    #![allow(missing_docs)]

    use near_sdk::json_types::U128;

    use super::*;

    #[near_sdk::ext_contract(ext_nep181)]
    pub trait Nep181 {
        fn nft_total_supply(&self) -> U128;
        fn nft_tokens(&self, from_index: Option<U128>, limit: Option<u32>) -> Vec<Token>;
        fn nft_supply_for_owner(&self, account_id: AccountId) -> U128;
        fn nft_tokens_for_owner(
            &self,
            account_id: AccountId,
            from_index: Option<U128>,
            limit: Option<u32>,
        ) -> Vec<Token>;
    }
}
