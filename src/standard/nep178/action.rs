//! NEP-178 actions.
//!
//! Used when calling various functions on [`Nep178Controller`]. Also used when
//! implementing [`Hook`]s for the NEP-178 component.

use super::*;
compat_use_borsh!(BorshSerialize);
use near_sdk::serde::Serialize;

compat_derive_serde_borsh! {[Serialize, BorshSerialize],
    /// NEP-178 approve action.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Nep178Approve<'a> {
        /// Token ID that the target account is being approved for.
        pub token_id: &'a TokenId,
        /// Account ID of the current owner of the token.
        pub current_owner_id: &'a AccountId,
        /// Account ID of the target account. This account will be able to
        /// transfer the token.
        pub account_id: &'a AccountId,
    }
}

compat_derive_serde_borsh! {[Serialize, BorshSerialize],
    /// NEP-178 revoke action.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Nep178Revoke<'a> {
        /// Token ID that the target account will no longer be able to transfer
        /// (approval revoked).
        pub token_id: &'a TokenId,
        /// Account ID of the current owner of the token.
        pub current_owner_id: &'a AccountId,
        /// Account ID of the target account. This account will no longer be able
        /// to transfer the token.
        pub account_id: &'a AccountId,
    }
}

compat_derive_serde_borsh! {[Serialize, BorshSerialize],
    /// NEP-178 revoke all action.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Nep178RevokeAll<'a> {
        /// Token ID that all approvals will be revoked from.
        pub token_id: &'a TokenId,
        /// Account ID of the current owner of the token.
        pub current_owner_id: &'a AccountId,
    }
}
