use std::{borrow::Cow, marker::PhantomData};

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, require, AccountId,
};
use serde::{Deserialize, Serialize};

use crate::approval::ApprovalState;

pub trait ExpireMultisigApprover {
    fn approve(account_id: &AccountId) -> Result<(), Cow<str>>;
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug)]
pub struct ExpireMultisigConfig<A: ExpireMultisigApprover> {
    pub threshold: u8,
    pub expire_approvals_after_blocks: u64,
    #[borsh_skip]
    #[serde(skip)]
    __approver: PhantomData<A>,
}

impl<A: ExpireMultisigApprover> ExpireMultisigConfig<A> {
    pub fn new(threshold: u8, expire_approvals_after_blocks: u64) -> Self {
        Self {
            threshold,
            expire_approvals_after_blocks,
            __approver: PhantomData,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct DatedApprovalRecord {
    pub account_id: AccountId,
    pub block_height: u64,
}

#[derive(BorshSerialize, BorshDeserialize, Default, Debug)]
pub struct ExpireMultisigApprovalState {
    pub approvals: Vec<DatedApprovalRecord>,
}

impl<A: ExpireMultisigApprover> ApprovalState<ExpireMultisigConfig<A>>
    for ExpireMultisigApprovalState
{
    fn is_approved(&self, config: &ExpireMultisigConfig<A>) -> bool {
        let validity_period_start = env::block_height() - config.expire_approvals_after_blocks;
        let valid_approvals = self
            .approvals
            .iter()
            .filter(|record| {
                let DatedApprovalRecord {
                    account_id,
                    block_height,
                } = record;
                *block_height >= validity_period_start && A::approve(account_id).is_ok()
            })
            .count();

        valid_approvals >= config.threshold as usize
    }

    fn try_approve(&mut self, _args: Option<String>, _config: &ExpireMultisigConfig<A>) {
        let predecessor = env::predecessor_account_id();

        A::approve(&predecessor).unwrap_or_else(|e| env::panic_str(&e));

        require!(
            self.approvals
                .iter()
                .find(|record| {
                    let DatedApprovalRecord { account_id, .. } = record;
                    &predecessor == account_id
                })
                .is_none(),
            "Already approved by this account",
        );

        self.approvals.push(DatedApprovalRecord {
            account_id: predecessor,
            block_height: env::block_height(),
        });
    }
}
