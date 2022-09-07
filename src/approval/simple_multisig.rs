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

/// An AccountAuthorizer gatekeeps which accounts are eligible to submit approvals
/// to an ApprovalManager
pub trait AccountAuthorizer {
    /// Why can this account not be authorized?
    type AuthorizationError;

    /// Determines whether an account ID is allowed to submit an approval
    fn is_account_authorized(account_id: &AccountId) -> Result<(), Self::AuthorizationError>;
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

/// If a request has expired, some actions may not be performed
#[derive(Error, Clone, Debug)]
#[error("Validity period exceeded")]
pub struct RequestExpiredError;

/// Why might a simple multisig approval attempt fail?
#[derive(Error, Clone, Debug)]
pub enum ApprovalError {
    /// The account has already approved this action request
    #[error("Already approved by this account")]
    AlreadyApprovedByAccount,
    /// The request has expired and cannot be approved or executed
    #[error(transparent)]
    RequestExpired(#[from] RequestExpiredError),
}

/// Errors when evaluating a request for execution
#[derive(Error, Clone, Debug)]
pub enum ExecutionEligibilityError {
    /// The request does not have enough approvals
    #[error("Insufficient approvals on request: required {required} but only has {current}")]
    InsufficientApprovals {
        /// Current number of approvals
        current: usize,
        /// Required number of approvals
        required: usize,
    },
    /// The request has expired and cannot be approved or executed
    #[error(transparent)]
    RequestExpired(#[from] RequestExpiredError),
}

/// What errors may occur when removing a request?
#[derive(Error, Clone, Debug)]
pub enum RemovalError {
    /// Requests may not be removed while they are still valid
    #[error("Removal prohibited before expiration")]
    RequestStillValid,
}

impl<Au, Ac> ApprovalConfiguration<Ac, ApprovalState> for Configuration<Au>
where
    Au: AccountAuthorizer,
{
    type ApprovalError = ApprovalError;
    type RemovalError = RemovalError;
    type AuthorizationError = Au::AuthorizationError;
    type ExecutionEligibilityError = ExecutionEligibilityError;

    fn is_approved_for_execution(
        &self,
        action_request: &ActionRequest<Ac, ApprovalState>,
    ) -> Result<(), ExecutionEligibilityError> {
        if !self.is_within_validity_period(&action_request.approval_state) {
            return Err(RequestExpiredError.into());
        }

        let current = action_request.approval_state.approved_by.len();
        let required = self.threshold as usize;

        if current < required {
            return Err(ExecutionEligibilityError::InsufficientApprovals { current, required });
        }

        Ok(())
    }

    fn is_removable(
        &self,
        action_request: &ActionRequest<Ac, ApprovalState>,
    ) -> Result<(), Self::RemovalError> {
        if self.is_within_validity_period(&action_request.approval_state) {
            Err(RemovalError::RequestStillValid)
        } else {
            Ok(())
        }
    }

    fn is_account_authorized(
        &self,
        account_id: &AccountId,
        _action_request: &ActionRequest<Ac, ApprovalState>,
    ) -> Result<(), Self::AuthorizationError> {
        Au::is_account_authorized(account_id)
    }

    fn try_approve_with_authorized_account(
        &self,
        account_id: AccountId,
        action_request: &mut ActionRequest<Ac, ApprovalState>,
    ) -> Result<(), Self::ApprovalError> {
        if !self.is_within_validity_period(&action_request.approval_state) {
            return Err(RequestExpiredError.into());
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

/// Types used by near-contract-tools-macros
pub mod macro_types {
    use thiserror::Error;

    /// Account that attempted an action is missing a role
    #[derive(Error, Clone, Debug)]
    #[error("Missing role '{0}' required for this action")]
    pub struct MissingRole<R>(pub R);
}

#[cfg(test)]
mod tests {
    use near_sdk::{
        borsh::{self, BorshDeserialize, BorshSerialize},
        env, near_bindgen,
        test_utils::VMContextBuilder,
        testing_env, AccountId, BorshStorageKey,
    };
    use thiserror::Error;

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

    #[derive(Error, Clone, Debug)]
    #[error("Missing role: {0}")]
    struct MissingRole(&'static str);

    impl AccountAuthorizer for Contract {
        type AuthorizationError = MissingRole;

        fn is_account_authorized(account_id: &near_sdk::AccountId) -> Result<(), MissingRole> {
            if Self::has_role(account_id, &Role::Multisig) {
                Ok(())
            } else {
                Err(MissingRole("Multisig"))
            }
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
        assert!(Contract::is_approved_for_execution(request_id).is_err());

        predecessor(&alice);
        contract.approve(request_id);

        assert!(Contract::is_approved_for_execution(request_id).is_err());

        predecessor(&charlie);
        contract.approve(request_id);

        assert!(Contract::is_approved_for_execution(request_id).is_ok());

        predecessor(&bob);
        contract.approve(request_id);

        assert!(Contract::is_approved_for_execution(request_id).is_ok());

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
