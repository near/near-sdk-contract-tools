//! NEP-178 non-fungible token approval management implementation.
//!
//! Reference: <https://github.com/near/NEPs/blob/master/neps/nep-0178.md>
use std::{borrow::Cow, collections::HashMap, error::Error};

use near_sdk::{
    AccountId, AccountIdRef, BorshStorageKey, borsh::BorshSerialize, collections::UnorderedMap,
    near,
};

use crate::{
    DefaultStorageKey,
    hook::Hook,
    slot::Slot,
    standard::nep171::{
        CheckExternalTransfer, DefaultCheckExternalTransfer, LoadTokenMetadata, Nep171Controller,
        Nep171TransferAuthorization, TokenId,
        action::{Nep171Burn, Nep171Mint, Nep171Transfer},
        error::Nep171TransferError,
    },
};

pub mod action;
use action::*;
pub mod error;
use error::*;
// separate module with re-export because ext_contract doesn't play well with #![warn(missing_docs)]
mod ext;
pub use ext::*;

/// Type for approval IDs.
pub type ApprovalId = u32;
/// Maximum number of approvals per token.
pub const MAX_APPROVALS: u64 = 32;

/// NFT token approvals. Hooks are implemented on this struct.
#[derive(Debug)]
#[near]
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

impl<C: Nep178Controller> Hook<C, Nep171Mint<'_>> for TokenApprovals {}

impl<C: Nep178Controller> Hook<C, Nep171Transfer<'_>> for TokenApprovals {
    fn hook<R>(contract: &mut C, args: &Nep171Transfer<'_>, f: impl FnOnce(&mut C) -> R) -> R {
        let r = f(contract);
        contract.revoke_all_unchecked(&args.token_id);
        r
    }
}

impl<C: Nep178Controller> Hook<C, Nep171Burn<'_>> for TokenApprovals {
    fn hook<R>(contract: &mut C, args: &Nep171Burn<'_>, f: impl FnOnce(&mut C) -> R) -> R {
        let r = f(contract);
        for token_id in &args.token_ids {
            contract.revoke_all_unchecked(token_id);
        }
        r
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
                    contract.get_approval_id_for(&transfer.token_id, &transfer.sender_id);

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
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey<'a> {
    TokenApprovals(&'a TokenId),
    TokenApprovalsUnorderedMap(&'a TokenId),
}

/// Internal functions for [`Nep178Controller`].
pub trait Nep178ControllerInternal {
    /// Hook for approve operations.
    type ApproveHook: for<'a> Hook<Self, Nep178Approve<'a>>
    where
        Self: Sized;
    /// Hook for revoke operations.
    type RevokeHook: for<'a> Hook<Self, Nep178Revoke<'a>>
    where
        Self: Sized;
    /// Hook for revoke all operations.
    type RevokeAllHook: for<'a> Hook<Self, Nep178RevokeAll<'a>>
    where
        Self: Sized;

    /// Storage root.
    #[must_use]
    fn root() -> Slot<()> {
        Slot::root(DefaultStorageKey::Nep178)
    }

    /// Storage slot for token approvals.
    #[must_use]
    fn slot_token_approvals(token_id: &TokenId) -> Slot<TokenApprovals> {
        Self::root().field(StorageKey::TokenApprovals(token_id))
    }

    /// Storage slot for token approvals `UnorderedMap`.
    #[must_use]
    fn slot_token_approvals_unordered_map(
        token_id: &TokenId,
    ) -> Slot<UnorderedMap<AccountId, ApprovalId>> {
        Self::root().field(StorageKey::TokenApprovalsUnorderedMap(token_id))
    }
}

/// Functions for managing token approvals, NEP-178.
pub trait Nep178Controller {
    /// Hook for approve operations.
    type ApproveHook: for<'a> Hook<Self, Nep178Approve<'a>>
    where
        Self: Sized;
    /// Hook for revoke operations.
    type RevokeHook: for<'a> Hook<Self, Nep178Revoke<'a>>
    where
        Self: Sized;
    /// Hook for revoke all operations.
    type RevokeAllHook: for<'a> Hook<Self, Nep178RevokeAll<'a>>
    where
        Self: Sized;

    /// Approve a token for transfer by a delegated account.
    ///
    /// # Errors
    ///
    /// - If the acting account is not authorized to create approvals for the token.
    /// - If the target account is already approved for the token.
    /// - If the token exceeds the maximum number of approvals.
    fn approve(&mut self, action: &Nep178Approve<'_>) -> Result<ApprovalId, Nep178ApproveError>;

    /// Approve a token without checking if the account is already approved or
    /// if it exceeds the maximum number of approvals.
    fn approve_unchecked(&mut self, token_id: &TokenId, account_id: &AccountIdRef) -> ApprovalId;

    /// Revoke approval for an account to transfer token.
    ///
    /// # Errors
    ///
    /// - If the acting account is not authorized to revoke approvals for the token.
    /// - If the target account is not approved for the token.
    fn revoke(&mut self, action: &Nep178Revoke<'_>) -> Result<(), Nep178RevokeError>;

    /// Revoke approval for an account to transfer token without checking if
    /// the account is approved.
    fn revoke_unchecked(&mut self, token_id: &TokenId, account_id: &AccountIdRef);

    /// Revoke all approvals for a token.
    ///
    /// # Errors
    ///
    /// - If the acting account is not authorized to revoke approvals for the token.
    fn revoke_all(&mut self, action: &Nep178RevokeAll<'_>) -> Result<(), Nep178RevokeAllError>;

    /// Revoke all approvals for a token without checking current owner.
    fn revoke_all_unchecked(&mut self, token_id: &TokenId);

    /// Get the approval ID for an account, if it is approved for a token.
    fn get_approval_id_for(
        &self,
        token_id: &TokenId,
        account_id: &AccountIdRef,
    ) -> Option<ApprovalId>;

    /// Get the approvals for a token.
    fn get_approvals_for(&self, token_id: &TokenId) -> HashMap<AccountId, ApprovalId>;
}

impl<T: Nep178ControllerInternal + Nep171Controller> Nep178Controller for T {
    type ApproveHook = T::ApproveHook;
    type RevokeHook = T::RevokeHook;
    type RevokeAllHook = T::RevokeAllHook;

    fn approve_unchecked(&mut self, token_id: &TokenId, account_id: &AccountIdRef) -> ApprovalId {
        let mut slot = Self::slot_token_approvals(token_id);
        let mut approvals = slot.read().unwrap_or_else(|| TokenApprovals {
            next_approval_id: 0,
            accounts: UnorderedMap::new(Self::slot_token_approvals_unordered_map(token_id)),
        });
        let approval_id = approvals.next_approval_id;
        approvals.accounts.insert(&account_id.into(), &approval_id);
        approvals.next_approval_id += 1; // overflow unrealistic
        slot.write(&approvals);

        approval_id
    }

    fn approve(&mut self, action: &Nep178Approve<'_>) -> Result<ApprovalId, Nep178ApproveError> {
        // owner check
        if self.token_owner(&action.token_id).as_deref() != Some(action.current_owner_id.as_ref()) {
            return Err(UnauthorizedError {
                token_id: action.token_id.clone(),
                account_id: action.account_id.clone().into(),
            }
            .into());
        }

        let mut slot = Self::slot_token_approvals(&action.token_id);
        let mut approvals = slot.read().unwrap_or_else(|| TokenApprovals {
            next_approval_id: 0,
            accounts: UnorderedMap::new(Self::slot_token_approvals_unordered_map(&action.token_id)),
        });

        if approvals.accounts.len() >= MAX_APPROVALS {
            return Err(TooManyApprovalsError {
                token_id: action.token_id.clone(),
            }
            .into());
        }

        let approval_id = approvals.next_approval_id;
        if approvals
            .accounts
            .get(&AccountId::from(action.account_id.as_ref()))
            .is_some()
        {
            return Err(AccountAlreadyApprovedError {
                token_id: action.token_id.clone(),
                account_id: action.account_id.clone().into(),
            }
            .into());
        }

        Self::ApproveHook::hook(self, action, |_| {
            approvals
                .accounts
                .insert(&action.account_id.clone().into(), &approval_id);
            approvals.next_approval_id += 1; // overflow unrealistic
            slot.write(&approvals);

            Ok(approval_id)
        })
    }

    fn revoke_unchecked(&mut self, token_id: &TokenId, account_id: &AccountIdRef) {
        let mut slot = Self::slot_token_approvals(token_id);
        let Some(mut approvals) = slot.read() else {
            return;
        };

        let old = approvals.accounts.remove(&account_id.into());

        if old.is_some() {
            slot.write(&approvals);
        }
    }

    fn revoke(&mut self, action: &Nep178Revoke) -> Result<(), Nep178RevokeError> {
        // owner check
        if self.token_owner(&action.token_id).as_deref() != Some(action.current_owner_id.as_ref()) {
            return Err(UnauthorizedError {
                token_id: action.token_id.clone(),
                account_id: action.account_id.clone().into(),
            }
            .into());
        }

        let mut slot = Self::slot_token_approvals(&action.token_id);
        let mut approvals = slot.read().ok_or_else(|| AccountNotApprovedError {
            token_id: action.token_id.clone(),
            account_id: action.account_id.clone().into(),
        })?;

        if approvals
            .accounts
            .get(&AccountId::from(action.account_id.as_ref()))
            .is_none()
        {
            return Err(AccountNotApprovedError {
                token_id: action.token_id.clone(),
                account_id: action.account_id.clone().into(),
            }
            .into());
        }

        Self::RevokeHook::hook(self, action, |_| {
            approvals
                .accounts
                .remove(&AccountId::from(action.account_id.as_ref()));
            slot.write(&approvals);

            Ok(())
        })
    }

    fn revoke_all(&mut self, action: &Nep178RevokeAll) -> Result<(), Nep178RevokeAllError> {
        // owner check
        if self.token_owner(&action.token_id).as_deref() != Some(action.current_owner_id.as_ref()) {
            return Err(UnauthorizedError {
                token_id: action.token_id.clone(),
                account_id: action.current_owner_id.clone().into(),
            }
            .into());
        }

        Self::RevokeAllHook::hook(self, action, |contract| {
            contract.revoke_all_unchecked(&action.token_id);

            Ok(())
        })
    }

    fn revoke_all_unchecked(&mut self, token_id: &TokenId) {
        let mut slot = Self::slot_token_approvals(token_id);
        let Some(mut approvals) = slot.read() else {
            return;
        };

        if !approvals.accounts.is_empty() {
            approvals.accounts.clear();
            slot.write(&approvals);
        }
    }

    fn get_approval_id_for(
        &self,
        token_id: &TokenId,
        account_id: &AccountIdRef,
    ) -> Option<ApprovalId> {
        let slot = Self::slot_token_approvals(token_id);
        let approvals = slot.read()?;

        approvals.accounts.get(&account_id.into())
    }

    fn get_approvals_for(&self, token_id: &TokenId) -> HashMap<AccountId, ApprovalId> {
        let slot = Self::slot_token_approvals(token_id);
        let Some(approvals) = slot.read() else {
            return HashMap::default();
        };

        approvals.accounts.into_iter().collect()
    }
}
