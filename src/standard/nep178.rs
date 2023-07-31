//! NEP-178 non-fungible token approval management implementation.
//!
//! Reference: <https://github.com/near/NEPs/blob/master/neps/nep-0178.md>
use std::{collections::HashMap, error::Error};

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    serde::*,
    store::UnorderedMap,
    AccountId, BorshStorageKey,
};
use thiserror::Error;

use crate::{slot::Slot, standard::nep171::*, DefaultStorageKey};

pub use ext::*;

pub type ApprovalId = u32;
pub const MAX_APPROVALS: ApprovalId = 32;

/// Non-fungible token metadata.
#[derive(Serialize, BorshSerialize, BorshDeserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct TokenApprovals {
    #[serde(skip)]
    pub next_approval_id: ApprovalId,

    #[serde(flatten, serialize_with = "to_map")]
    /// The list of approved accounts.
    pub accounts: UnorderedMap<AccountId, ApprovalId>,
}

fn to_map<S: Serializer>(
    accounts: &UnorderedMap<AccountId, ApprovalId>,
    s: S,
) -> Result<S::Ok, S::Error> {
    s.collect_map(accounts.iter())
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

impl<C: Nep178Controller> Nep171Hook<C, ()> for TokenApprovals {
    fn before_nft_transfer(contract: &C, transfer: &Nep171Transfer) -> () {
        todo!()
    }

    fn after_nft_transfer(contract: &mut C, transfer: &Nep171Transfer, state: ()) {}
}

impl<C: Nep171Controller + Nep178Controller> CheckExternalTransfer<C> for TokenApprovals {
    fn check_external_transfer(
        contract: &C,
        transfer: &Nep171Transfer,
    ) -> Result<(), Nep171TransferError> {
        let normal_check = contract.check_transfer(
            &[transfer.token_id.to_string()],
            transfer.owner_id,
            transfer.sender_id,
            transfer.receiver_id,
        );

        match normal_check {
            Ok(()) => Ok(()),
            Err(e @ Nep171TransferError::SenderNotApproved(_)) => {
                let approval_id = if let Some(id) = transfer.approval_id {
                    id
                } else {
                    return Err(e);
                };

                let saved_approvals = contract.get_approvals_for(transfer.token_id);

                if saved_approvals.get(transfer.sender_id) == Some(&approval_id) {
                    Ok(())
                } else {
                    Err(e)
                }
            }
            e => e,
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

#[derive(Error, Debug)]
pub enum Nep178ApproveError {
    #[error("Account {account_id} is already approved for token {token_id}.")]
    AccountAlreadyApproved {
        token_id: TokenId,
        account_id: AccountId,
    },
    #[error(
        "Too many approvals for token {token_id}, maximum is {}.",
        MAX_APPROVALS
    )]
    TooManyApprovals { token_id: TokenId },
}

#[derive(Error, Debug)]
pub enum Nep178RevokeError {
    #[error("Account {account_id} is not approved for token {token_id}")]
    AccountNotApproved {
        token_id: TokenId,
        account_id: AccountId,
    },
}

/// Functions for managing non-fungible tokens with attached metadata, NEP-178.
pub trait Nep178Controller {
    /// Approve a token for transfer by a delegated account.
    fn approve(
        &mut self,
        token_id: &TokenId,
        account_id: &AccountId,
    ) -> Result<ApprovalId, Nep178ApproveError>;

    /// Approve a token without checking if the account is already approved or
    /// if it exceeds the maximum number of approvals.
    fn approve_unchecked(&mut self, token_id: &TokenId, account_id: &AccountId) -> ApprovalId;

    /// Revoke approval for an account to transfer token.
    fn revoke(
        &mut self,
        token_id: &TokenId,
        account_id: &AccountId,
    ) -> Result<(), Nep178RevokeError>;

    /// Revoke approval for an account to transfer token without checking if
    /// the account is approved.
    fn revoke_unchecked(&mut self, token_id: &TokenId, account_id: &AccountId);

    /// Revoke all approvals for a token.
    fn revoke_all(&mut self, token_id: &TokenId);

    /// Get the approval ID for an account, if it is approved for a token.
    fn get_approval_id_for(&self, token_id: &TokenId, account_id: &AccountId)
        -> Option<ApprovalId>;

    /// Get the approvals for a token.
    fn get_approvals_for(&self, token_id: &TokenId) -> HashMap<AccountId, ApprovalId>;
}

impl<T: Nep178ControllerInternal + Nep171Controller> Nep178Controller for T {
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
        account_id: &AccountId,
    ) -> Result<ApprovalId, Nep178ApproveError> {
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
        if approvals
            .accounts
            .insert(account_id.clone(), approval_id)
            .is_some()
        {
            return Err(Nep178ApproveError::AccountAlreadyApproved {
                token_id: token_id.clone(),
                account_id: account_id.clone(),
            });
        }
        approvals.next_approval_id += 1; // overflow unrealistic
        slot.write(&approvals);

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
        account_id: &AccountId,
    ) -> Result<(), Nep178RevokeError> {
        let mut slot = Self::slot_token_approvals(token_id);
        let mut approvals = slot
            .read()
            .ok_or_else(|| Nep178RevokeError::AccountNotApproved {
                token_id: token_id.clone(),
                account_id: account_id.clone(),
            })?;

        approvals
            .accounts
            .remove(account_id)
            .ok_or(Nep178RevokeError::AccountNotApproved {
                token_id: token_id.clone(),
                account_id: account_id.clone(),
            })?;

        slot.write(&approvals);

        Ok(())
    }

    fn revoke_all(&mut self, token_id: &TokenId) {
        let mut slot = Self::slot_token_approvals(token_id);
        let approvals = match slot.read() {
            Some(approvals) => approvals,
            None => return,
        };

        slot.write(&TokenApprovals {
            next_approval_id: approvals.next_approval_id,
            accounts: UnorderedMap::new(Self::slot_token_approvals_unordered_map(token_id)),
        });
    }

    fn get_approval_id_for(
        &self,
        token_id: &TokenId,
        account_id: &AccountId,
    ) -> Option<ApprovalId> {
        let slot = Self::slot_token_approvals(token_id);
        let approvals = match slot.read() {
            Some(approvals) => approvals,
            None => return None,
        };

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

// separate module with re-export because ext_contract doesn't play well with #![warn(missing_docs)]
mod ext {
    #![allow(missing_docs)]

    use near_sdk::PromiseOrValue;

    use super::*;

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
}
