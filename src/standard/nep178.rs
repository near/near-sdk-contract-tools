//! NEP-178 non-fungible token approval management implementation.
//!
//! Reference: <https://github.com/near/NEPs/blob/master/neps/nep-0178.md>
use std::{collections::HashMap, error::Error};

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    store::UnorderedMap,
    AccountId, BorshStorageKey,
};
use thiserror::Error;

use crate::{hook::Hook, slot::Slot, standard::nep171::*, DefaultStorageKey};

pub use ext::*;

/// Type for approval IDs.
pub type ApprovalId = u32;
/// Maximum number of approvals per token.
pub const MAX_APPROVALS: ApprovalId = 32;

/// Non-fungible token metadata.
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct TokenApprovals {
    /// The next approval ID to use. Only incremented.
    pub next_approval_id: ApprovalId,

    /// The list of approved accounts.
    pub accounts: UnorderedMap<AccountId, ApprovalId>,
}

impl<C: Nep178Controller> LoadTokenMetadata<C> for TokenApprovals {
    fn load(
        contract: &C,
        token_id: &TokenId,
        metadata: &mut std::collections::HashMap<String, near_sdk::serde_json::Value>,
    ) -> Result<(), Box<dyn Error>> {
        metadata.insert(
            "approved_account_ids".to_string(),
            near_sdk::serde_json::to_value(contract.get_approvals_for(token_id))?,
        );
        Ok(())
    }
}

impl<C: Nep178Controller> Hook<C, Nep171Mint<'_>> for TokenApprovals {
    type State = ();
}

impl<C: Nep178Controller> Hook<C, Nep171Transfer<'_>> for TokenApprovals {
    type State = ();

    fn after(contract: &mut C, transfer: &Nep171Transfer<'_>, _: ()) {
        contract.revoke_all_unchecked(transfer.token_id);
    }
}

impl<C: Nep178Controller> Hook<C, Nep171Burn<'_>> for TokenApprovals {
    type State = ();

    fn after(contract: &mut C, transfer: &Nep171Burn<'_>, _: ()) {
        for token_id in transfer.token_ids {
            contract.revoke_all_unchecked(token_id);
        }
    }
}

impl<C: Nep171Controller + Nep178Controller> CheckExternalTransfer<C> for TokenApprovals {
    fn check_external_transfer(
        contract: &C,
        transfer: &Nep171Transfer,
    ) -> Result<AccountId, Nep171TransferError> {
        let normal_check =
            DefaultCheckExternalTransfer::check_external_transfer(contract, transfer);

        match (&transfer.authorization, normal_check) {
            (_, r @ Ok(_)) => r,
            (
                Nep171TransferAuthorization::ApprovalId(approval_id),
                Err(Nep171TransferError::SenderNotApproved(s)),
            ) => {
                let saved_approval =
                    contract.get_approval_id_for(transfer.token_id, transfer.sender_id);

                if saved_approval == Some(*approval_id) {
                    Ok(s.owner_id)
                } else {
                    Err(s.into())
                }
            }
            (_, e @ Err(_)) => e,
        }
    }
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey<'a> {
    TokenApprovals(&'a TokenId),
    TokenApprovalsUnorderedMap(&'a TokenId),
}

/// Internal functions for [`Nep178Controller`].
pub trait Nep178ControllerInternal {
    /// Lifecycle hooks for NEP-178.
    type Hook: Nep178Hook<Self>
    where
        Self: Sized;

    /// Storage root.
    fn root() -> Slot<()> {
        Slot::root(DefaultStorageKey::Nep178)
    }

    /// Storage slot for token approvals.
    fn slot_token_approvals(token_id: &TokenId) -> Slot<TokenApprovals> {
        Self::root().field(StorageKey::TokenApprovals(token_id))
    }

    /// Storage slot for token approvals `UnorderedMap`.
    fn slot_token_approvals_unordered_map(
        token_id: &TokenId,
    ) -> Slot<UnorderedMap<AccountId, ApprovalId>> {
        Self::root().field(StorageKey::TokenApprovalsUnorderedMap(token_id))
    }
}

/// Errors that can occur when managing non-fungible token approvals.
#[derive(Error, Debug)]
pub enum Nep178ApproveError {
    /// The account is not authorized to approve the token.
    #[error("Account `{account_id}` cannot create approvals for token `{token_id}`.")]
    Unauthorized {
        /// The token ID.
        token_id: TokenId,
        /// The unauthorized account ID.
        account_id: AccountId,
    },
    /// The account is already approved for the token.
    #[error("Account {account_id} is already approved for token {token_id}.")]
    AccountAlreadyApproved {
        /// The token ID.
        token_id: TokenId,
        /// The account ID that has already been approved.
        account_id: AccountId,
    },
    /// The token has too many approvals.
    #[error(
        "Too many approvals for token {token_id}, maximum is {}.",
        MAX_APPROVALS
    )]
    TooManyApprovals {
        /// The token ID.
        token_id: TokenId,
    },
}

/// Errors that can occur when revoking non-fungible token approvals.
#[derive(Error, Debug)]
pub enum Nep178RevokeError {
    /// The account is not authorized to revoke approvals for the token.
    #[error("Account `{account_id}` is cannot revoke approvals for token `{token_id}`.")]
    Unauthorized {
        /// The token ID.
        token_id: TokenId,
        /// The unauthorized account ID.
        account_id: AccountId,
    },
    /// The account is not approved for the token.
    #[error("Account {account_id} is not approved for token {token_id}")]
    AccountNotApproved {
        /// The token ID.
        token_id: TokenId,
        /// The account ID that is not approved.
        account_id: AccountId,
    },
}

/// Errors that can occur when revoking all approvals for a non-fungible token.
#[derive(Error, Debug)]
pub enum Nep178RevokeAllError {
    /// The account is not authorized to revoke approvals for the token.
    #[error("Account `{account_id}` is cannot revoke approvals for token `{token_id}`.")]
    Unauthorized {
        /// The token ID.
        token_id: TokenId,
        /// The unauthorized account ID.
        account_id: AccountId,
    },
}

/// Functions for managing non-fungible tokens with attached metadata, NEP-178.
pub trait Nep178Controller {
    /// Lifecycle hooks for NEP-178.
    type Hook: Nep178Hook<Self>
    where
        Self: Sized;

    /// Approve a token for transfer by a delegated account.
    fn approve(
        &mut self,
        token_id: &TokenId,
        current_owner_id: &AccountId,
        account_id: &AccountId,
    ) -> Result<ApprovalId, Nep178ApproveError>;

    /// Approve a token without checking if the account is already approved or
    /// if it exceeds the maximum number of approvals.
    fn approve_unchecked(&mut self, token_id: &TokenId, account_id: &AccountId) -> ApprovalId;

    /// Revoke approval for an account to transfer token.
    fn revoke(
        &mut self,
        token_id: &TokenId,
        current_owner_id: &AccountId,
        account_id: &AccountId,
    ) -> Result<(), Nep178RevokeError>;

    /// Revoke approval for an account to transfer token without checking if
    /// the account is approved.
    fn revoke_unchecked(&mut self, token_id: &TokenId, account_id: &AccountId);

    /// Revoke all approvals for a token.
    fn revoke_all(
        &mut self,
        token_id: &TokenId,
        current_owner_id: &AccountId,
    ) -> Result<(), Nep178RevokeAllError>;

    /// Revoke all approvals for a token without checking current owner.
    fn revoke_all_unchecked(&mut self, token_id: &TokenId);

    /// Get the approval ID for an account, if it is approved for a token.
    fn get_approval_id_for(&self, token_id: &TokenId, account_id: &AccountId)
        -> Option<ApprovalId>;

    /// Get the approvals for a token.
    fn get_approvals_for(&self, token_id: &TokenId) -> HashMap<AccountId, ApprovalId>;
}

impl<T: Nep178ControllerInternal + Nep171Controller> Nep178Controller for T {
    type Hook = <Self as Nep178ControllerInternal>::Hook;

    fn approve_unchecked(&mut self, token_id: &TokenId, account_id: &AccountId) -> ApprovalId {
        let mut slot = Self::slot_token_approvals(token_id);
        let mut approvals = slot.read().unwrap_or_else(|| TokenApprovals {
            next_approval_id: 0,
            accounts: UnorderedMap::new(Self::slot_token_approvals_unordered_map(token_id)),
        });
        let approval_id = approvals.next_approval_id;
        approvals.accounts.insert(account_id.clone(), approval_id);
        approvals.next_approval_id += 1; // overflow unrealistic
        slot.write(&approvals);

        approval_id
    }

    fn approve(
        &mut self,
        token_id: &TokenId,
        current_owner_id: &AccountId,
        account_id: &AccountId,
    ) -> Result<ApprovalId, Nep178ApproveError> {
        // owner check
        if self.token_owner(token_id).as_ref() != Some(current_owner_id) {
            return Err(Nep178ApproveError::Unauthorized {
                token_id: token_id.clone(),
                account_id: account_id.clone(),
            });
        }

        let mut slot = Self::slot_token_approvals(token_id);
        let mut approvals = slot.read().unwrap_or_else(|| TokenApprovals {
            next_approval_id: 0,
            accounts: UnorderedMap::new(Self::slot_token_approvals_unordered_map(token_id)),
        });

        if approvals.accounts.len() >= MAX_APPROVALS {
            return Err(Nep178ApproveError::TooManyApprovals {
                token_id: token_id.clone(),
            });
        }

        let approval_id = approvals.next_approval_id;
        if approvals.accounts.contains_key(account_id) {
            return Err(Nep178ApproveError::AccountAlreadyApproved {
                token_id: token_id.clone(),
                account_id: account_id.clone(),
            });
        }

        let hook_state = Self::Hook::before_nft_approve(self, token_id, account_id);

        approvals.accounts.insert(account_id.clone(), approval_id);
        approvals.next_approval_id += 1; // overflow unrealistic
        slot.write(&approvals);

        Self::Hook::after_nft_approve(self, token_id, account_id, &approval_id, hook_state);

        Ok(approval_id)
    }

    fn revoke_unchecked(&mut self, token_id: &TokenId, account_id: &AccountId) {
        let mut slot = Self::slot_token_approvals(token_id);
        let mut approvals = match slot.read() {
            Some(approvals) => approvals,
            None => return,
        };

        let old = approvals.accounts.remove(account_id);

        if old.is_some() {
            slot.write(&approvals);
        }
    }

    fn revoke(
        &mut self,
        token_id: &TokenId,
        current_owner_id: &AccountId,
        account_id: &AccountId,
    ) -> Result<(), Nep178RevokeError> {
        // owner check
        if self.token_owner(token_id).as_ref() != Some(current_owner_id) {
            return Err(Nep178RevokeError::Unauthorized {
                token_id: token_id.clone(),
                account_id: account_id.clone(),
            });
        }

        let mut slot = Self::slot_token_approvals(token_id);
        let mut approvals = slot
            .read()
            .ok_or_else(|| Nep178RevokeError::AccountNotApproved {
                token_id: token_id.clone(),
                account_id: account_id.clone(),
            })?;

        if !approvals.accounts.contains_key(account_id) {
            return Err(Nep178RevokeError::AccountNotApproved {
                token_id: token_id.clone(),
                account_id: account_id.clone(),
            });
        }

        let hook_state = Self::Hook::before_nft_revoke(self, token_id, account_id);

        approvals.accounts.remove(account_id);
        slot.write(&approvals);

        Self::Hook::after_nft_revoke(self, token_id, account_id, hook_state);

        Ok(())
    }

    fn revoke_all(
        &mut self,
        token_id: &TokenId,
        current_owner_id: &AccountId,
    ) -> Result<(), Nep178RevokeAllError> {
        // owner check
        if self.token_owner(token_id).as_ref() != Some(current_owner_id) {
            return Err(Nep178RevokeAllError::Unauthorized {
                token_id: token_id.clone(),
                account_id: current_owner_id.clone(),
            });
        }

        let hook_state = Self::Hook::before_nft_revoke_all(self, token_id);

        self.revoke_all_unchecked(token_id);

        Self::Hook::after_nft_revoke_all(self, token_id, hook_state);

        Ok(())
    }

    fn revoke_all_unchecked(&mut self, token_id: &TokenId) {
        let mut slot = Self::slot_token_approvals(token_id);
        let mut approvals = match slot.read() {
            Some(approvals) => approvals,
            None => return,
        };

        if !approvals.accounts.is_empty() {
            approvals.accounts.clear();
            slot.write(&approvals);
        }
    }

    fn get_approval_id_for(
        &self,
        token_id: &TokenId,
        account_id: &AccountId,
    ) -> Option<ApprovalId> {
        let slot = Self::slot_token_approvals(token_id);
        let approvals = slot.read()?;

        approvals.accounts.get(account_id).copied()
    }

    fn get_approvals_for(&self, token_id: &TokenId) -> HashMap<AccountId, ApprovalId> {
        let slot = Self::slot_token_approvals(token_id);
        let approvals = match slot.read() {
            Some(approvals) => approvals,
            None => return HashMap::default(),
        };

        approvals
            .accounts
            .into_iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }
}

/// Hooks for NEP-178.
pub trait Nep178Hook<C = Self> {
    /// State passed from [`Nep178Hook::before_nft_approve`] to
    /// [`Nep178Hook::after_nft_approve`].
    type NftApproveState;

    /// State passed from [`Nep178Hook::before_nft_revoke`] to
    /// [`Nep178Hook::after_nft_revoke`].
    type NftRevokeState;

    /// State passed from [`Nep178Hook::before_nft_revoke_all`] to
    /// [`Nep178Hook::after_nft_revoke_all`].
    type NftRevokeAllState;

    /// Called before a token is approved for transfer.
    fn before_nft_approve(
        contract: &C,
        token_id: &TokenId,
        account_id: &AccountId,
    ) -> Self::NftApproveState;

    /// Called after a token is approved for transfer.
    fn after_nft_approve(
        contract: &mut C,
        token_id: &TokenId,
        account_id: &AccountId,
        approval_id: &ApprovalId,
        state: Self::NftApproveState,
    );

    /// Called before a token approval is revoked.
    fn before_nft_revoke(
        contract: &C,
        token_id: &TokenId,
        account_id: &AccountId,
    ) -> Self::NftRevokeState;

    /// Called after a token approval is revoked.
    fn after_nft_revoke(
        contract: &mut C,
        token_id: &TokenId,
        account_id: &AccountId,
        state: Self::NftRevokeState,
    );

    /// Called before all approvals for a token are revoked.
    fn before_nft_revoke_all(contract: &C, token_id: &TokenId) -> Self::NftRevokeAllState;

    /// Called after all approvals for a token are revoked.
    fn after_nft_revoke_all(contract: &mut C, token_id: &TokenId, state: Self::NftRevokeAllState);
}

impl<C> Nep178Hook<C> for () {
    type NftApproveState = ();
    type NftRevokeState = ();
    type NftRevokeAllState = ();

    fn before_nft_approve(_contract: &C, _token_id: &TokenId, _account_id: &AccountId) {}

    fn after_nft_approve(
        _contract: &mut C,
        _token_id: &TokenId,
        _account_id: &AccountId,
        _approval_id: &ApprovalId,
        _: (),
    ) {
    }

    fn before_nft_revoke(_contract: &C, _token_id: &TokenId, _account_id: &AccountId) {}

    fn after_nft_revoke(_contract: &mut C, _token_id: &TokenId, _account_id: &AccountId, _: ()) {}

    fn before_nft_revoke_all(_contract: &C, _token_id: &TokenId) {}

    fn after_nft_revoke_all(_contract: &mut C, _token_id: &TokenId, _: ()) {}
}

impl<T, U, C> Nep178Hook<C> for (T, U)
where
    T: Nep178Hook<C>,
    U: Nep178Hook<C>,
{
    type NftApproveState = (T::NftApproveState, U::NftApproveState);
    type NftRevokeState = (T::NftRevokeState, U::NftRevokeState);
    type NftRevokeAllState = (T::NftRevokeAllState, U::NftRevokeAllState);

    fn before_nft_approve(
        contract: &C,
        token_id: &TokenId,
        account_id: &AccountId,
    ) -> Self::NftApproveState {
        (
            T::before_nft_approve(contract, token_id, account_id),
            U::before_nft_approve(contract, token_id, account_id),
        )
    }

    fn after_nft_approve(
        contract: &mut C,
        token_id: &TokenId,
        account_id: &AccountId,
        approval_id: &ApprovalId,
        (t_state, u_state): Self::NftApproveState,
    ) {
        T::after_nft_approve(contract, token_id, account_id, approval_id, t_state);
        U::after_nft_approve(contract, token_id, account_id, approval_id, u_state);
    }

    fn before_nft_revoke(
        contract: &C,
        token_id: &TokenId,
        account_id: &AccountId,
    ) -> Self::NftRevokeState {
        (
            T::before_nft_revoke(contract, token_id, account_id),
            U::before_nft_revoke(contract, token_id, account_id),
        )
    }

    fn after_nft_revoke(
        contract: &mut C,
        token_id: &TokenId,
        account_id: &AccountId,
        (t_state, u_state): Self::NftRevokeState,
    ) {
        T::after_nft_revoke(contract, token_id, account_id, t_state);
        U::after_nft_revoke(contract, token_id, account_id, u_state);
    }

    fn before_nft_revoke_all(contract: &C, token_id: &TokenId) -> Self::NftRevokeAllState {
        (
            T::before_nft_revoke_all(contract, token_id),
            U::before_nft_revoke_all(contract, token_id),
        )
    }

    fn after_nft_revoke_all(
        contract: &mut C,
        token_id: &TokenId,
        (t_state, u_state): Self::NftRevokeAllState,
    ) {
        T::after_nft_revoke_all(contract, token_id, t_state);
        U::after_nft_revoke_all(contract, token_id, u_state);
    }
}

/// Alternative to [`Nep178Hook`] that is simpler to implement. There is a
/// blanket implementation of [`Nep178Hook`] for all types that implement
/// [`SimpleNep178Hook`].
pub trait SimpleNep178Hook {
    /// Called before a token is approved for transfer.
    fn before_nft_approve(&self, _token_id: &TokenId, _account_id: &AccountId) {}
    /// Called after a token is approved for transfer.
    fn after_nft_approve(
        &mut self,
        _token_id: &TokenId,
        _account_id: &AccountId,
        _approval_id: &ApprovalId,
    ) {
    }

    /// Called before a token approval is revoked.
    fn before_nft_revoke(&self, _token_id: &TokenId, _account_id: &AccountId) {}
    /// Called after a token approval is revoked.
    fn after_nft_revoke(&mut self, _token_id: &TokenId, _account_id: &AccountId) {}

    /// Called before all approvals for a token are revoked.
    fn before_nft_revoke_all(&self, _token_id: &TokenId) {}
    /// Called after all approvals for a token are revoked.
    fn after_nft_revoke_all(&mut self, _token_id: &TokenId) {}
}

impl<T: SimpleNep178Hook> Nep178Hook<Self> for T {
    type NftApproveState = ();

    type NftRevokeState = ();

    type NftRevokeAllState = ();

    fn before_nft_approve(contract: &Self, token_id: &TokenId, account_id: &AccountId) {
        <T as SimpleNep178Hook>::before_nft_approve(contract, token_id, account_id);
    }

    fn after_nft_approve(
        contract: &mut Self,
        token_id: &TokenId,
        account_id: &AccountId,
        approval_id: &ApprovalId,
        _: (),
    ) {
        <T as SimpleNep178Hook>::after_nft_approve(contract, token_id, account_id, approval_id);
    }

    fn before_nft_revoke(contract: &Self, token_id: &TokenId, account_id: &AccountId) {
        <T as SimpleNep178Hook>::before_nft_revoke(contract, token_id, account_id);
    }

    fn after_nft_revoke(contract: &mut Self, token_id: &TokenId, account_id: &AccountId, _: ()) {
        <T as SimpleNep178Hook>::after_nft_revoke(contract, token_id, account_id);
    }

    fn before_nft_revoke_all(contract: &Self, token_id: &TokenId) {
        <T as SimpleNep178Hook>::before_nft_revoke_all(contract, token_id);
    }

    fn after_nft_revoke_all(contract: &mut Self, token_id: &TokenId, _: ()) {
        <T as SimpleNep178Hook>::after_nft_revoke_all(contract, token_id);
    }
}

// separate module with re-export because ext_contract doesn't play well with #![warn(missing_docs)]
mod ext {
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
}
