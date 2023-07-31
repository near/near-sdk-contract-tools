#![allow(missing_docs)]

use std::collections::HashMap;

use near_sdk::{ext_contract, AccountId, PromiseOrValue};

use super::TokenId;

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
    fn nft_token(&self, token_id: TokenId) -> Option<super::Token>;
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
