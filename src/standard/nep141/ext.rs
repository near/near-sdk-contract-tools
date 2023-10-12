#![allow(missing_docs)]

use near_sdk::{ext_contract, json_types::U128, AccountId, Promise, PromiseOrValue};

/// A contract that may be the recipient of an `ft_transfer_call` function
/// call.
#[ext_contract(ext_nep141_receiver)]
pub trait Nep141Receiver {
    /// Function that is called in an `ft_transfer_call` promise chain.
    /// Returns the number of tokens "used", that is, those that will be kept
    /// in the receiving contract's account. (The contract will attempt to
    /// refund the difference from `amount` to the original sender.)
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

/// Fungible token contract callback after `ft_transfer_call` execution.
#[ext_contract(ext_nep141_resolver)]
pub trait Nep141Resolver {
    /// Callback, last in `ft_transfer_call` promise chain. Returns the amount
    /// of tokens refunded to the original sender.
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}

/// Externally-accessible NEP-141-compatible fungible token interface.
#[ext_contract(ext_nep141)]
pub trait Nep141 {
    /// Performs a token transfer
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);

    /// Performs a token transfer, then initiates a promise chain that calls
    /// `ft_on_transfer` on the receiving account, followed by
    /// `ft_resolve_transfer` on the original token contract (this contract).
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> Promise;

    /// Returns the current total amount of tokens tracked by the contract
    fn ft_total_supply(&self) -> U128;

    /// Returns the amount of tokens controlled by `account_id`
    fn ft_balance_of(&self, account_id: AccountId) -> U128;
}
