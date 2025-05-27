#![allow(missing_docs)]

use std::collections::HashMap;

use near_sdk::{
    ext_contract, json_types::U128, near, AccountId, AccountIdRef, Promise, PromiseOrValue,
};

use super::{ApprovalId, TokenId};

#[derive(Debug, Clone)]
#[near(serializers = [json, borsh])]
pub struct Token {
    pub token_id: TokenId,
    pub owner_id: Option<AccountId>,
}

/// A contract that may be the recipient of an `mt_transfer_call` function
/// call.
#[ext_contract(ext_nep245_receiver)]
pub trait Nep245Receiver {
    /// Take some action after receiving a multi token.
    ///
    /// Returns the number of unused tokens in string form. For instance, if `amounts`
    /// is `["10"]` but only 9 are needed, it will return `["1"]`.
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>>;
}

#[derive(Debug, Clone)]
#[near(serializers = [borsh, json])]
pub struct MtResolveTransferApproval(pub AccountId, pub ApprovalId, pub U128);

impl MtResolveTransferApproval {
    #[must_use]
    pub fn owner_id(&self) -> &AccountIdRef {
        &self.0
    }

    #[must_use]
    pub fn approval_id(&self) -> ApprovalId {
        self.1
    }

    #[must_use]
    pub fn amount(&self) -> u128 {
        u128::from(self.2)
    }
}

/// Multi token contract callback after `mt_transfer_call` execution.
#[ext_contract(ext_nep245_resolver)]
pub trait Nep245Resolver {
    fn mt_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
    ) -> Vec<U128>;
}

#[derive(Debug, Clone)]
#[near(serializers = [json, borsh])]
pub struct MtTransferApproval(pub AccountId, pub u32);

impl MtTransferApproval {
    #[must_use]
    pub fn owner_id(&self) -> &AccountIdRef {
        &self.0
    }

    #[must_use]
    pub fn approval_id(&self) -> ApprovalId {
        self.1
    }
}

#[derive(Debug, Clone)]
#[near(serializers = [json, borsh])]
pub struct Approval {
    pub amount: U128,
    pub approval_id: ApprovalId,
}

#[derive(Debug, Clone)]
#[near(serializers = [json, borsh])]
pub struct TokenApproval {
    pub approval_owner_id: AccountId,
    pub approved_account_ids: HashMap<AccountId, Approval>,
}

/// Externally-accessible NEP-245-compatible multi token interface.
#[ext_contract(ext_nep245)]
pub trait Nep245 {
    /// Simple transfer. Transfer a given `token_id` from current owner to
    /// `receiver_id`.
    ///
    /// Requirements:
    /// * Caller of the method must attach a deposit of 1 yoctoNEAR for security purposes.
    /// * Caller must have greater than or equal to the `amount` being requested.
    /// * Contract MUST panic if called by someone other than token owner or,
    ///   if using approval management, one of the approved accounts.
    /// * `approval_id` is for use with approval management extension.
    /// * If using approval management, contract MUST nullify approved accounts on
    ///   successful transfer.
    ///
    /// Arguments:
    /// * `receiver_id`: the valid NEAR account receiving the token.
    /// * `token_id`: the token to transfer.
    /// * `amount`: the number of tokens to transfer.
    /// * `approval` (optional): a tuple of the form `(owner_id, approval_id)`:
    ///      * `owner_id`: the valid Near account that owns the tokens.
    ///      * `approval_id`: the expected approval ID.
    /// * `memo` (optional): for use cases that may benefit from indexing or
    ///      providing information for a transfer.
    fn mt_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<MtTransferApproval>,
        memo: Option<String>,
    );

    /// Simple batch transfer. Transfer a given `token_ids` from current owner
    /// to `receiver_id`.
    ///
    /// Requirements:
    /// * Caller of the method must attach a deposit of 1 yoctoNEAR for
    ///     security purposes.
    /// * Caller must have greater than or equal to the `amounts` being
    ///     requested for the given `token_ids`.
    /// * Contract MUST panic if called by someone other than token owner or,
    ///     if using approval management, one of the approved accounts
    /// * `approval_id` is for use with approval management extension.
    /// * If using approval management, contract MUST nullify approved accounts
    ///     on successful transfer.
    /// * Contract MUST panic if the length of `token_ids` is not equal to the
    ///     length of `amounts`.
    /// * Contract MUST panic if `approval_ids` is not `null` and its length
    ///     does not equal the length of `token_ids`.
    ///
    /// Arguments:
    /// * `receiver_id`: the valid NEAR account receiving the token.
    /// * `token_ids`: the tokens to transfer.
    /// * `amounts`: the number of tokens to transfer.
    /// * `approvals` (optional): is an array of expected `approval` per
    ///     `token_ids`. If a `token_id` does not have a corresponding
    ///     `approval` then the entry in the array must be `null`.
    ///
    ///     `approval` is a tuple of the form `(owner_id, approval_id)`:
    ///     * `owner_id`: the valid NEAR account that owns the tokens.
    ///     * `approval_id` is the expected approval ID.
    /// * `memo` (optional): for use cases that may benefit from indexing or
    ///    providing information for a transfer.
    fn mt_batch_transfer(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<MtTransferApproval>>>,
        memo: Option<String>,
    );

    /// Transfer token and call a method on a receiver contract. A successful
    /// workflow will end in a success execution outcome to the callback on the
    /// multi token contract at the method `mt_resolve_transfer`.
    ///
    /// You can think of this as being similar to attaching native NEAR tokens
    /// to a function call. It allows you to attach any multi token contract's
    /// token in a call to a receiver contract.
    ///
    /// Requirements:
    /// * Caller of the method must attach a deposit of 1 yoctoNEAR for security
    ///     purposes.
    /// * Caller must have greater than or equal to the `amount` being requested.
    /// * Contract MUST panic if called by someone other than token owner or, if
    ///     using approval management, one of the approved accounts.
    /// * The receiving contract must implement `mt_on_transfer` according to
    ///     the standard. If it does not, MT contract's `mt_resolve_transfer`
    ///     MUST deal with the resulting failed cross-contract call and roll
    ///     back the transfer.
    /// * Contract MUST implement the behavior described in
    ///     `mt_resolve_transfer`.
    /// * `approval_id` is for use with approval management extension.
    /// * If using approval management, contract MUST nullify approved accounts
    ///     on successful transfer.
    ///
    /// Arguments:
    /// * `receiver_id`: the valid NEAR account receiving the token.
    /// * `token_id`: the token to send.
    /// * `amount`: the number of tokens to transfer.
    /// * `owner_id`: the valid NEAR account that owns the token.
    /// * `approval` (optional): is a tuple of the form `(owner_id, approval_id)`:
    ///     * `owner_id` is the valid NEAR account that owns the tokens.
    ///     * `approval_id` is the expected approval ID.
    /// * `memo` (optional): for use cases that may benefit from indexing or
    ///     providing information for a transfer.
    /// * `msg`: specifies information needed by the receiving contract in
    ///    order to properly handle the transfer. Can indicate both a function to
    ///    call and the parameters to pass to that function.
    fn mt_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        amount: U128,
        approval: Option<MtTransferApproval>,
        memo: Option<String>,
        msg: String,
    ) -> Promise;

    /// Transfer tokens and call a method on a receiver contract. A successful
    /// workflow will end in a success execution outcome to the callback on the MT
    /// contract at the method `mt_resolve_transfer`.
    ///
    /// You can think of this as being similar to attaching native NEAR tokens to a
    /// function call. It allows you to attach any Multi Token, token in a call to a
    /// receiver contract.
    ///
    /// Requirements:
    /// * Caller of the method must attach a deposit of 1 yoctoNEAR for
    ///     security purposes.
    /// * Caller must have greater than or equal to the `amount` being requested.
    /// * Contract MUST panic if called by someone other than token owner or,
    ///     if using approval management, one of the approved accounts.
    /// * The receiving contract must implement `mt_on_transfer` according to
    ///     the standard. If it does not, the multi token contract's
    ///     `mt_resolve_transfer` MUST handle the resulting failed
    ///     cross-contract call and roll back the transfer.
    /// * Contract MUST implement the behavior described in
    ///     `mt_resolve_transfer`.
    /// * `approval_id` is for use with approval management extension.
    /// * If using approval management, contract MUST nullify approved accounts
    ///     on successful transfer.
    /// * Contract MUST panic if the length of `token_ids` is not equal to the
    ///     length of `amounts`.
    /// * Contract MUST panic if `approval_ids` is not `null` and its length
    ///     does not equal the length of `token_ids`.
    ///
    /// Arguments:
    /// * `receiver_id`: the valid NEAR account receiving the token.
    /// * `token_ids`: the tokens to transfer.
    /// * `amounts`: the number of tokens to transfer.
    /// * `approvals` (optional): an array of expected `approval` for
    ///     each entry in `token_ids`. If a `token_id` does not have a
    ///     corresponding `approval` then the entry in the array must be
    ///     `null`.
    ///
    ///     `approval` is a tuple of the form `(owner_id, approval_id)`:
    ///     * `owner_id`: the valid NEAR account that owns the tokens.
    ///     * `approval_id`: the expected approval ID.
    /// * `memo` (optional): for use cases that may benefit from indexing or
    ///    providing information for a transfer.
    /// * `msg`: specifies information needed by the receiving contract in
    ///    order to properly handle the transfer. Can indicate both a function to
    ///    call and the parameters to pass to that function.
    fn mt_batch_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        approvals: Option<Vec<Option<MtTransferApproval>>>,
        memo: Option<String>,
        msg: String,
    ) -> Promise;

    /// Returns the tokens with the given `token_ids` or `null` if no such token.
    fn mt_token(&self, token_ids: Vec<TokenId>) -> Vec<Option<Token>>;

    /// Returns the balance of an account for the given `token_id`.
    ///
    /// Arguments:
    /// - `account_id`: the NEAR account that owns the token.
    /// - `token_id`: the token to retrieve the balance from.
    fn mt_balance_of(&self, account_id: AccountId, token_id: TokenId) -> U128;

    /// Returns the balances of an account for the given `token_ids`.
    ///
    /// Arguments:
    /// * `account_id`: the NEAR account that owns the tokens.
    /// * `token_ids`: the tokens to retrieve the balance from.
    fn mt_batch_balance_of(&self, account_id: AccountId, token_ids: Vec<TokenId>) -> Vec<U128>;

    /// Returns the token supply with the given `token_id` or `null` if no such token exists.
    fn mt_supply(&self, token_id: TokenId) -> Option<U128>;

    /// Returns the token supplies with the given `token_ids`, a string value is returned or `null`
    /// if no such token exists.
    fn mt_batch_supply(&self, token_ids: Vec<TokenId>) -> Vec<Option<U128>>;
}

#[ext_contract(ext_nep245_approval)]
pub trait Nep245Approval {
    /// Add an approved account for a specific set of tokens.
    ///
    /// Requirements:
    /// * Caller of the method must attach a deposit of at least 1 yoctoNEAR for
    ///   security purposes.
    /// * Contract MAY require caller to attach larger deposit, to cover cost of
    ///   storing approver data.
    /// * Contract MUST panic if called by someone other than token owner.
    /// * Contract MUST panic if addition would cause `mt_revoke_all` to exceed
    ///   single-block gas limit.
    /// * Contract MUST increment approval ID even if re-approving an account.
    /// * If successfully approved or if had already been approved, and if `msg` is
    ///   present, contract MUST call `mt_on_approve` on `account_id`.
    ///
    /// Arguments:
    /// * `token_ids`: the token ids for which to add an approval.
    /// * `account_id`: the account to add to `approved_account_ids`
    /// * `amounts`: the number of tokens to approve for transfer.
    /// * `msg`: optional string to be passed to `mt_on_approve`.
    ///
    /// Returns void, if no `msg` given. Otherwise, returns promise call to
    /// `mt_on_approve`, which can resolve with whatever it wants.
    fn mt_approve(
        &mut self,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        account_id: AccountId,
        msg: Option<String>,
    ) -> PromiseOrValue<()>;

    /// Revoke an approved account for a specific token.
    ///
    /// Requirements
    /// * Caller of the method must attach a deposit of 1 yoctoNEAR for security
    ///   purposes.
    /// * If contract requires > 1 yoctoNEAR deposit on `mt_approve`, contract
    ///   MUST refund associated storage deposit when owner revokes approval.
    /// * Contract MUST panic if called by someone other than token owner.
    ///
    /// Arguments:
    /// * `token_ids`: the token for which to revoke approved_account_ids.
    /// * `account_id`: the account to remove from `approvals`.
    fn mt_revoke(&mut self, token_ids: Vec<TokenId>, account_id: AccountId);

    /// Revoke all approved accounts for a specific token.
    ///
    /// Requirements
    /// * Caller of the method must attach a deposit of 1 yoctoNEAR for security
    ///     purposes.
    /// * If contract requires > 1 yoctoNEAR deposit on `mt_approve`, contract
    ///     MUST refund all associated storage deposit when owner revokes
    ///     `approved_account_ids`.
    /// * Contract MUST panic if called by someone other than token owner.
    ///
    /// Arguments:
    /// * `token_ids`: the token ids with `approved_account_ids` to revoke.
    fn mt_revoke_all(&mut self, token_ids: Vec<TokenId>);

    /// Check if tokens are approved for transfer by a given account, optionally
    /// checking an `approval_id`.
    ///
    /// Requirements:
    /// * Contract MUST panic if `approval_ids` is not `null` and the length of
    ///     `approval_ids` is not equal to `token_ids`.
    ///
    /// Arguments:
    /// * `token_ids`: the tokens for which to check an approval.
    /// * `approved_account_id`: the account to check the existence of in
    ///     approved_account_ids`.
    /// * `amounts`: specify the positionally corresponding amount for the `token_id`
    ///     that at least must be approved.
    /// * `approval_ids`: an optional array of approval IDs to check against
    ///     current approval IDs for given account and `token_ids`.
    ///
    /// Returns:
    /// * If `approval_ids` is given, `true` if `approved_account_id` is
    ///     approved with given `approval_id` and has at least the amount
    ///     specified approved.
    /// * Otherwise, `true` if `approved_account_id` is in list of approved
    ///     accounts and has at least the amount specified approved.
    ///
    /// It returns `false` for all other states.
    fn mt_is_approved(
        &self,
        token_ids: Vec<TokenId>,
        approved_account_id: AccountId,
        amounts: Vec<U128>,
        approval_ids: Option<Vec<ApprovalId>>,
    ) -> bool;

    /// Get a the list of approvals for a given `token_id` and `account_id`.
    ///
    /// Arguments:
    /// * `token_id`: the token for which to check an approval.
    /// * `account_id`: the account to retrieve approvals for.
    fn mt_token_approval(&self, token_id: TokenId, account_id: AccountId) -> TokenApproval;

    /// Get a list of all approvals for a given token ID.
    ///
    /// Arguments:
    /// * `from_index`: the starting index of tokens to return, default `"0"`.
    /// * `limit`: the maximum number of tokens to return.
    ///
    /// Returns an array of [`TokenApproval`] objects or an empty array if there
    /// are no approvals.
    fn mt_token_approvals(
        &self,
        token_id: TokenId,
        from_index: Option<U128>,
        limit: Option<u32>,
    ) -> Vec<TokenApproval>;
}
