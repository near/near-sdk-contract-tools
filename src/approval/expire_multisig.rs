use std::{borrow::Cow, marker::PhantomData};

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, require, AccountId,
};
use serde::{Deserialize, Serialize};

use crate::approval::{Approval, ApprovalState};

pub type ExpireMultisig<Action, Approver> =
    Approval<Action, ExpireMultisigApprovalState, ExpireMultisigConfig<Approver>>;

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

#[derive(Debug)]
pub struct DatedApprovalRecord {
    account_id: AccountId,
    block_height: u64,
}

#[derive(Default, Debug)]
pub struct ExpireMultisigApprovalState {
    pub approved_by: Vec<DatedApprovalRecord>,
}

impl<A: ExpireMultisigApprover> ApprovalState<ExpireMultisigConfig<A>>
    for ExpireMultisigApprovalState
{
    fn is_approved(&self, config: &ExpireMultisigConfig<A>) -> bool {
        let validity_period_start = env::block_height() - config.expire_approvals_after_blocks;
        self.approved_by
            .iter()
            .filter(|record| {
                let DatedApprovalRecord {
                    account_id,
                    block_height,
                } = record;
                *block_height >= validity_period_start && A::approve(account_id).is_ok()
            })
            .count()
            >= config.threshold as usize
    }

    fn attempt_approval(&mut self, _args: Option<String>, _config: &ExpireMultisigConfig<A>) {
        let predecessor = env::predecessor_account_id();

        A::approve(&predecessor).unwrap_or_else(|e| env::panic_str(&e));

        require!(
            self.approved_by
                .iter()
                .find(|DatedApprovalRecord { account_id, .. }| account_id == &predecessor)
                .is_none(),
            "Already approved by this account",
        );

        self.approved_by.push(DatedApprovalRecord {
            account_id: predecessor,
            block_height: env::block_height(),
        });
    }

    fn attempt_rejection(
        &mut self,
        _args: Option<String>,
        _config: &ExpireMultisigConfig<A>,
    ) -> bool {
        let predecessor = env::predecessor_account_id();

        A::approve(&predecessor).unwrap_or_else(|e| env::panic_str(&e));

        self.approved_by
            .retain(|DatedApprovalRecord { account_id, .. }| account_id != &predecessor);

        self.approved_by.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::{
        borsh::{self, BorshDeserialize, BorshSerialize},
        env, near_bindgen,
        test_utils::VMContextBuilder,
        testing_env, AccountId, BorshStorageKey,
    };

    use crate::{approval::Approval, near_contract_tools, rbac::Rbac, Rbac};

    use super::{ExpireMultisig, ExpireMultisigApprover, ExpireMultisigConfig};

    #[derive(BorshSerialize, BorshDeserialize)]
    enum Action {
        SayHello,
        SayGoodbye,
    }

    impl crate::approval::Action for Action {
        type Output = &'static str;

        fn execute(self) -> Self::Output {
            match self {
                Self::SayHello => "hello",
                Self::SayGoodbye => "goodbye",
            }
        }
    }

    #[derive(BorshSerialize, BorshStorageKey)]
    enum Role {
        Multisig,
    }

    #[derive(Rbac)]
    #[rbac(roles = "Role")]
    #[near_bindgen]
    struct Contract {
        pub approval: ExpireMultisig<Action, Self>,
    }

    impl ExpireMultisigApprover for Contract {
        fn approve(account_id: &near_sdk::AccountId) -> Result<(), std::borrow::Cow<str>> {
            if Self::has_role(account_id, &Role::Multisig) {
                Ok(())
            } else {
                Err("Must have multisig role".into())
            }
        }
    }

    #[near_bindgen]
    impl Contract {
        #[init]
        pub fn new() -> Self {
            Self {
                approval: Approval::new(ExpireMultisigConfig::new(2, 10)),
            }
        }

        pub fn obtain_multisig_permission(&mut self) {
            self.add_role(&env::predecessor_account_id(), &Role::Multisig);
        }

        pub fn create(&mut self, say_hello: bool) -> u32 {
            self.require_role(&Role::Multisig);

            let action = if say_hello {
                Action::SayHello
            } else {
                Action::SayGoodbye
            };

            let request_id = self.approval.add_request(action);

            request_id
        }

        pub fn approve(&mut self, request_id: u32) {
            self.approval.attempt_approval(request_id, None);
        }

        pub fn reject(&mut self, request_id: u32) {
            self.approval.attempt_rejection(request_id, None);
        }

        pub fn execute(&mut self, request_id: u32) -> &'static str {
            self.approval.attempt_execution(request_id)
        }
    }

    fn predecessor(account_id: &AccountId) {
        let mut context = VMContextBuilder::new();
        context.predecessor_account_id(account_id.clone());
        testing_env!(context.build());
    }

    #[test]
    fn test() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob_acct".parse().unwrap();
        let charlie: AccountId = "charlie".parse().unwrap();

        let mut contract = Contract::new();

        predecessor(&alice);
        contract.obtain_multisig_permission();
        predecessor(&bob);
        contract.obtain_multisig_permission();
        predecessor(&charlie);
        contract.obtain_multisig_permission();

        let request_id = contract.create(true);

        assert_eq!(request_id, 0);
        assert!(!contract.approval.is_approved(request_id));

        predecessor(&alice);
        contract.approve(request_id);

        assert!(!contract.approval.is_approved(request_id));

        predecessor(&charlie);
        contract.approve(request_id);

        assert!(contract.approval.is_approved(request_id));

        assert_eq!(contract.execute(request_id), "hello");
    }
}
