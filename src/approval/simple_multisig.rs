//! Simple multi-signature wallet component. Generic over approvable actions.
//! Use with NativeTransactionAction for multisig over native transactions.

use std::marker::PhantomData;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, AccountId,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{ActionRequest, ApprovalConfiguration};

/// An AccountApprover gatekeeps which accounts are eligible to submit approvals
/// to an ApprovalManager
pub trait AccountAuthorizer {
    /// Determines whether an account ID is allowed to submit an approval
    fn is_account_authorized(account_id: &AccountId) -> bool;
}

/// M (threshold) of N approval scheme
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
pub struct Configuration<Au: AccountAuthorizer> {
    /// How many approvals are required?
    pub threshold: u8,
    /// A request cannot be executed, and can be deleted by any
    /// approval-eligible member after this period has elapsed.
    /// 0 = perpetual validity, no deletion
    pub validity_period_nanoseconds: u64,
    #[borsh_skip]
    #[serde(skip)]
    _authorizer: PhantomData<Au>,
}

impl<Au: AccountAuthorizer> Configuration<Au> {
    /// Create an approval scheme with the given threshold
    pub fn new(threshold: u8, validity_period_nanoseconds: u64) -> Self {
        Self {
            threshold,
            validity_period_nanoseconds,
            _authorizer: PhantomData,
        }
    }

    /// Is the given approval state still considered valid?
    pub fn is_within_validity_period(&self, approval_state: &ApprovalState) -> bool {
        if self.validity_period_nanoseconds == 0 {
            true
        } else {
            env::block_timestamp()
                .checked_sub(approval_state.created_at_nanoseconds)
                .unwrap() // inconsistent state if a request timestamp is in the future
                < self.validity_period_nanoseconds
        }
    }
}

/// Approval state for simple multisig
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug)]
pub struct ApprovalState {
    /// List of accounts that have approved an action thus far
    pub approved_by: Vec<AccountId>,
    /// Network timestamp when the request was created
    pub created_at_nanoseconds: u64,
}

impl Default for ApprovalState {
    fn default() -> Self {
        Self::new()
    }
}

impl ApprovalState {
    /// Creates an ApprovalState with the current network timestamp
    pub fn new() -> Self {
        Self {
            approved_by: Vec::new(),
            created_at_nanoseconds: env::block_timestamp(),
        }
    }
}

/// Why might a simple multisig approval attempt fail?
#[derive(Error, Clone, Debug)]
pub enum ApprovalError {
    /// The account has already approved this action request
    #[error("Already approved by this account")]
    AlreadyApprovedByAccount,
    /// The request has expired and cannot be approved or executed
    #[error("Validity period exceeded")]
    RequestExpired,
}

impl<Au, Ac> ApprovalConfiguration<Ac, ApprovalState> for Configuration<Au>
where
    Au: AccountAuthorizer,
{
    type Error = ApprovalError;

    fn is_approved_for_execution(&self, action_request: &ActionRequest<Ac, ApprovalState>) -> bool {
        self.is_within_validity_period(&action_request.approval_state)
            && action_request.approval_state.approved_by.len() >= self.threshold as usize
    }

    fn is_removable(&self, action_request: &ActionRequest<Ac, ApprovalState>) -> bool {
        !self.is_within_validity_period(&action_request.approval_state)
    }

    fn is_account_authorized(
        &self,
        account_id: &AccountId,
        _action_request: &ActionRequest<Ac, ApprovalState>,
    ) -> bool {
        Au::is_account_authorized(account_id)
    }

    fn try_approve_with_authorized_account(
        &self,
        account_id: AccountId,
        action_request: &mut ActionRequest<Ac, ApprovalState>,
    ) -> Result<(), Self::Error> {
        if !self.is_within_validity_period(&action_request.approval_state) {
            return Err(ApprovalError::RequestExpired);
        }

        if action_request
            .approval_state
            .approved_by
            .contains(&account_id)
        {
            return Err(ApprovalError::AlreadyApprovedByAccount);
        }

        action_request.approval_state.approved_by.push(account_id);

        Ok(())
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

    use crate::{
        approval::{
            simple_multisig::{AccountAuthorizer, ApprovalState, Configuration},
            ApprovalManager,
        },
        near_contract_tools,
        rbac::Rbac,
        slot::Slot,
        Rbac,
    };

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

    #[derive(Rbac, Debug, BorshSerialize, BorshDeserialize)]
    #[rbac(roles = "Role")]
    #[near_bindgen]
    struct Contract {}

    impl ApprovalManager<Action, ApprovalState, Configuration<Self>> for Contract {
        fn root() -> Slot<()> {
            Slot::new(b"m")
        }
    }

    impl AccountAuthorizer for Contract {
        fn is_account_authorized(account_id: &near_sdk::AccountId) -> bool {
            Self::has_role(account_id, &Role::Multisig)
        }
    }

    #[near_bindgen]
    impl Contract {
        #[init]
        pub fn new() -> Self {
            <Self as ApprovalManager<_, _, _>>::init(Configuration::new(2, 10000));
            Self {}
        }

        pub fn obtain_multisig_permission(&mut self) {
            self.add_role(&env::predecessor_account_id(), &Role::Multisig);
        }

        pub fn create(&mut self, say_hello: bool) -> u32 {
            let action = if say_hello {
                Action::SayHello
            } else {
                Action::SayGoodbye
            };

            let request_id = self.create_request(action, ApprovalState::new()).unwrap();

            request_id
        }

        pub fn approve(&mut self, request_id: u32) {
            self.approve_request(request_id).unwrap();
        }

        pub fn execute(&mut self, request_id: u32) -> &'static str {
            self.execute_request(request_id).unwrap()
        }

        pub fn remove(&mut self, request_id: u32) {
            self.remove_request(request_id).unwrap()
        }
    }

    fn predecessor(account_id: &AccountId) {
        let mut context = VMContextBuilder::new();
        context.predecessor_account_id(account_id.clone());
        testing_env!(context.build());
    }

    #[test]
    fn successful_approval() {
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
        assert!(!Contract::is_approved(request_id));

        predecessor(&alice);
        contract.approve(request_id);

        assert!(!Contract::is_approved(request_id));

        predecessor(&charlie);
        contract.approve(request_id);

        assert!(Contract::is_approved(request_id));

        predecessor(&bob);
        contract.approve(request_id);

        assert!(Contract::is_approved(request_id));

        assert_eq!(contract.execute(request_id), "hello");
    }

    #[test]
    fn successful_removal() {
        let alice: AccountId = "alice".parse().unwrap();

        let mut contract = Contract::new();

        predecessor(&alice);
        contract.obtain_multisig_permission();

        let request_id = contract.create(true);

        contract.approve(request_id);

        let created_at = Contract::get_request(request_id)
            .unwrap()
            .approval_state
            .created_at_nanoseconds;

        let mut context = VMContextBuilder::new();
        context
            .predecessor_account_id(alice.clone())
            .block_timestamp(created_at + 10000);
        testing_env!(context.build());

        contract.remove(request_id);
    }

    #[test]
    #[should_panic = "RemovalNotAllowed"]
    fn unsuccessful_removal_not_expired() {
        let alice: AccountId = "alice".parse().unwrap();

        let mut contract = Contract::new();

        predecessor(&alice);
        contract.obtain_multisig_permission();

        let request_id = contract.create(true);

        contract.approve(request_id);

        let created_at = Contract::get_request(request_id)
            .unwrap()
            .approval_state
            .created_at_nanoseconds;

        let mut context = VMContextBuilder::new();
        context
            .predecessor_account_id(alice.clone())
            .block_timestamp(created_at + 9999);
        testing_env!(context.build());

        contract.remove(request_id);
    }

    #[test]
    #[should_panic = "UnauthorizedAccount"]
    fn unsuccessful_removal_no_permission() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob_acct".parse().unwrap();

        let mut contract = Contract::new();

        predecessor(&alice);
        contract.obtain_multisig_permission();

        let request_id = contract.create(true);

        contract.approve(request_id);

        let created_at = Contract::get_request(request_id)
            .unwrap()
            .approval_state
            .created_at_nanoseconds;

        let mut context = VMContextBuilder::new();
        context
            .predecessor_account_id(bob.clone())
            .block_timestamp(created_at + 10000);
        testing_env!(context.build());

        contract.remove(request_id);
    }
}
