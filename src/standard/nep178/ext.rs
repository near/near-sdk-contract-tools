#![allow(missing_docs)]

use near_sdk::PromiseOrValue;

use super::*;

/// NEP-178 external interface.
///
/// See <https://github.com/near/NEPs/blob/master/neps/nep-0178.md#interface> for more details.
#[near_sdk::ext_contract(ext_nep178)]
pub trait Nep178 {
    fn nft_approve(
        &mut self,
        token_id: TokenId,
        account_id: AccountId,
        msg: Option<String>,
    ) -> PromiseOrValue<()>;

    fn nft_revoke(&mut self, token_id: TokenId, account_id: AccountId);

    fn nft_revoke_all(&mut self, token_id: TokenId);

    fn nft_is_approved(
        &self,
        token_id: TokenId,
        approved_account_id: AccountId,
        approval_id: Option<ApprovalId>,
    ) -> bool;
}

/// NEP-178 receiver interface.
///
/// Respond to notification that contract has been granted approval for a token.
///
/// See <https://github.com/near/NEPs/blob/master/neps/nep-0178.md#approved-account-contract-interface> for more details.
#[near_sdk::ext_contract(ext_nep178_receiver)]
pub trait Nep178Receiver {
    fn nft_on_approve(
        &mut self,
        token_id: TokenId,
        owner_id: AccountId,
        approval_id: ApprovalId,
        msg: String,
    );
}
