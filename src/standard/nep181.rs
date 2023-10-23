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

use crate::{hook::Hook, slot::Slot, standard::nep171::*, DefaultStorageKey};

pub use ext::*;

/// Extension hook for [`Nep171Controller`].
pub struct TokenEnumeration;

impl<C: Nep171Controller + Nep181Controller> Hook<C, Nep171Mint<'_>> for TokenEnumeration {
    fn hook<R>(contract: &mut C, args: &Nep171Mint<'_>, f: impl FnOnce(&mut C) -> R) -> R {
        let r = f(contract);
        contract.add_tokens_to_enumeration(args.token_ids, args.receiver_id);
        r
    }
}

impl<C: Nep171Controller + Nep181Controller> Hook<C, Nep171Transfer<'_>> for TokenEnumeration {
    fn hook<R>(contract: &mut C, args: &Nep171Transfer<'_>, f: impl FnOnce(&mut C) -> R) -> R {
        let r = f(contract);
        let owner_id = match args.authorization {
            Nep171TransferAuthorization::Owner => Cow::Borrowed(args.sender_id),
            Nep171TransferAuthorization::ApprovalId(_) => Cow::Owned(contract.token_owner(args.token_id).unwrap_or_else(|| {
                env::panic_str(&format!("Inconsistent state: Enumeration reconciliation should only run after a token has been transferred, but token {} does not exist.", args.token_id))
            })),
        };

        contract.transfer_token_enumeration(
            std::array::from_ref(args.token_id),
            owner_id.as_ref(),
            args.receiver_id,
        );
        r
    }
}

impl<C: Nep171Controller + Nep181Controller> Hook<C, Nep171Burn<'_>> for TokenEnumeration {
    fn hook<R>(contract: &mut C, args: &Nep171Burn<'_>, f: impl FnOnce(&mut C) -> R) -> R {
        let r = f(contract);
        contract.remove_tokens_from_enumeration(args.token_ids, args.owner_id);
        r
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

    /// Storage slot for all tokens.
    fn slot_tokens() -> Slot<UnorderedSet<TokenId>> {
        Self::root().field(StorageKey::Tokens)
    }

    /// Storage slot for tokens owned by an account.
    fn slot_owner_tokens(owner_id: &AccountId) -> Slot<UnorderedSet<TokenId>> {
        Self::root().field(StorageKey::OwnerTokens(owner_id))
    }
}

/// Functions for managing non-fungible tokens with attached metadata, NEP-181.
pub trait Nep181Controller {
    /// Add tokens to enumeration.
    ///
    /// # Warning
    ///
    /// Does not perform consistency checks. May cause inconsistent state if
    /// the same token ID is added to the enumeration multiple times.
    fn add_tokens_to_enumeration(&mut self, token_ids: &[TokenId], owner_id: &AccountId);

    /// Remove tokens from enumeration.
    ///
    /// # Warning
    ///
    /// Does not perform consistency checks. May cause inconsistent state if
    /// any of the token IDs are not currently enumerated (owned) by `owner_id`.
    fn remove_tokens_from_enumeration(&mut self, token_ids: &[TokenId], owner_id: &AccountId);

    /// Transfer tokens between owners.
    ///
    /// # Warning
    ///
    /// Does not perform consistency checks. May cause inconsistent state if
    /// any of the token IDs are not currently enumerated (owned) by
    /// `from_owner_id`, or have not previously been added to enumeration via
    /// [`Nep181Controller::add_tokens_to_enumeration`].
    fn transfer_token_enumeration(
        &mut self,
        token_ids: &[TokenId],
        from_owner_id: &AccountId,
        to_owner_id: &AccountId,
    );

    /// Total number of tokens in enumeration.
    fn total_enumerated_tokens(&self) -> u128;

    /// Execute a function with a reference to the set of all tokens.
    fn with_tokens<T>(&self, f: impl FnOnce(&UnorderedSet<TokenId>) -> T) -> T;

    /// Execute a function with a reference to the set of tokens owned by an
    /// account.
    fn with_tokens_for_owner<T>(
        &self,
        owner_id: &AccountId,
        f: impl FnOnce(&UnorderedSet<TokenId>) -> T,
    ) -> T;
}

impl<T: Nep181ControllerInternal + Nep171Controller> Nep181Controller for T {
    fn add_tokens_to_enumeration(&mut self, token_ids: &[TokenId], owner_id: &AccountId) {
        let mut all_tokens_slot = Self::slot_tokens();
        let mut all_tokens = all_tokens_slot
            .read()
            .unwrap_or_else(|| UnorderedSet::new(StorageKey::Tokens));

        all_tokens.extend(token_ids.iter().cloned());

        all_tokens_slot.write(&all_tokens);

        let mut owner_tokens_slot = Self::slot_owner_tokens(owner_id);
        let mut owner_tokens = owner_tokens_slot
            .read()
            .unwrap_or_else(|| UnorderedSet::new(StorageKey::OwnerTokens(owner_id)));

        owner_tokens.extend(token_ids.iter().cloned());

        owner_tokens_slot.write(&owner_tokens);
    }

    fn remove_tokens_from_enumeration(&mut self, token_ids: &[TokenId], owner_id: &AccountId) {
        let mut all_tokens_slot = Self::slot_tokens();
        if let Some(mut all_tokens) = all_tokens_slot.read() {
            for token_id in token_ids {
                all_tokens.remove(token_id);
            }
            all_tokens_slot.write(&all_tokens);
        }

        let mut owner_tokens_slot = Self::slot_owner_tokens(owner_id);
        if let Some(mut owner_tokens) = owner_tokens_slot.read() {
            for token_id in token_ids {
                owner_tokens.remove(token_id);
            }
            owner_tokens_slot.write(&owner_tokens);
        }
    }

    fn transfer_token_enumeration(
        &mut self,
        token_ids: &[TokenId],
        from_owner_id: &AccountId,
        to_owner_id: &AccountId,
    ) {
        let mut from_owner_tokens_slot = Self::slot_owner_tokens(from_owner_id);
        if let Some(mut from_owner_tokens) = from_owner_tokens_slot.read() {
            for token_id in token_ids {
                from_owner_tokens.remove(token_id);
            }
            from_owner_tokens_slot.write(&from_owner_tokens);
        }

        let mut to_owner_tokens_slot = Self::slot_owner_tokens(to_owner_id);
        let mut to_owner_tokens = to_owner_tokens_slot
            .read()
            .unwrap_or_else(|| UnorderedSet::new(StorageKey::OwnerTokens(to_owner_id)));

        to_owner_tokens.extend(token_ids.iter().cloned());

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
