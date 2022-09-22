//! Queue and approve actions

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, require, AccountId, BorshStorageKey,
};
use serde::{Deserialize, Serialize};
use snafu::{Error, Snafu};

use crate::slot::Slot;

/// Error message emitted when the component is used before it is initialized
pub const NOT_INITIALIZED: &str = "init must be called before use";
/// Error message emitted when the init function is called multiple times
pub const ALREADY_INITIALIZED: &str = "init can only be called once";

pub mod native_transaction_action;
pub mod simple_multisig;

/// Actions can be executed after they are approved
pub trait Action<Cont: ?Sized> {
    /// Return type of the action. Useful if the action creates a `Promise`, for example.
    type Output;
    /// Perform the action. One time only.
    fn execute(self, contract: &mut Cont) -> Self::Output;
}

/// Defines the operating parameters for an ApprovalManager and performs
/// approvals
pub trait ApprovalConfiguration<A, S> {
    /// Errors when approving a request
    type ApprovalError: Error;
    /// Errors when removing a request
    type RemovalError: Error;
    /// Errors when authorizing an account
    type AuthorizationError: Error;
    /// Errors when evaluating a request for execution candidacy
    type ExecutionEligibilityError: Error;

    /// Has the request reached full approval?
    fn is_approved_for_execution(
        &self,
        action_request: &ActionRequest<A, S>,
    ) -> Result<(), Self::ExecutionEligibilityError>;

    /// Can this request be removed by an allowed account?
    fn is_removable(&self, action_request: &ActionRequest<A, S>) -> Result<(), Self::RemovalError>;

    /// Is the account allowed to execute, approve, or remove this request?
    fn is_account_authorized(
        &self,
        account_id: &AccountId,
        action_request: &ActionRequest<A, S>,
    ) -> Result<(), Self::AuthorizationError>;

    /// Modify action_request.approval_state in-place to increase approval
    fn try_approve_with_authorized_account(
        &self,
        account_id: AccountId,
        action_request: &mut ActionRequest<A, S>,
    ) -> Result<(), Self::ApprovalError>;
}

/// An action request is composed of an action that will be executed when the
/// associated approval state is satisfied
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug)]
pub struct ActionRequest<A, S> {
    /// The action that will be executed when the approval state is
    /// fulfilled
    pub action: A,
    /// The associated approval state
    pub approval_state: S,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum ApprovalStorageKey {
    NextRequestId,
    Config,
    Request(u32),
}

/// The account is ineligile to perform an action for some reason
#[derive(Snafu, Clone, Debug)]
#[snafu(display("Unauthorized account: '{account_id}'"))]
pub struct UnauthorizedAccountError<AuthErr: Error + 'static> {
    /// This account ID is unauthorized
    pub account_id: AccountId,
    /// Error source
    pub source: AuthErr,
}

/// Top-level errors that may occur when attempting to approve a request
#[derive(Snafu, Clone, Debug)]
pub enum ApprovalError<AuthErr: Error + 'static, AppErr: Error + 'static> {
    /// The account is not allowed to act on requests
    #[snafu(context(false))]
    UnauthorizedAccount {
        /// Error source
        source: UnauthorizedAccountError<AuthErr>,
    },
    /// The approval function encountered another error
    #[snafu(display("Approval error"))]
    ApprovalError {
        /// Error source
        source: AppErr,
    },
}

/// Errors that may occur when trying to execute a request
#[derive(Snafu, Clone, Debug)]
pub enum ExecutionError<AuthErr: Error + 'static, ExecErr: Error + 'static> {
    /// The account is not allowed to act on requests
    #[snafu(context(false))]
    UnauthorizedAccount {
        /// Error source
        source: UnauthorizedAccountError<AuthErr>,
    },
    /// Unapproved requests cannot be executed
    #[snafu(display("Request not approved"))]
    ExecutionEligibility {
        /// Error source
        source: ExecErr,
    },
}

/// Errors that may occur when trying to create a request
#[derive(Snafu, Clone, Debug)]
pub enum CreationError<AuthErr: Error + 'static> {
    /// The account is not allowed to act on requests
    #[snafu(context(false))]
    UnauthorizedAccount {
        /// Error source
        source: UnauthorizedAccountError<AuthErr>,
    },
}

/// Errors that may occur when trying to remove a request
#[derive(Snafu, Clone, Debug)]
pub enum RemovalError<AuthErr: Error + 'static, RemErr: Error + 'static> {
    /// The account is not allowed to act on requests
    #[snafu(context(false))]
    UnauthorizedAccount {
        /// Error source
        source: UnauthorizedAccountError<AuthErr>,
    },
    /// This request is not (yet?) allowed to be removed
    #[snafu(display("Removal not allowed"))]
    RemovalNotAllowed {
        /// Error source
        source: RemErr,
    },
}

/// Collection of action requests that manages their approval state and
/// execution
pub trait ApprovalManager<A, S, C>
where
    A: Action<Self> + BorshSerialize + BorshDeserialize,
    S: BorshSerialize + BorshDeserialize + Serialize,
    C: ApprovalConfiguration<A, S> + BorshDeserialize + BorshSerialize,
{
    /// Storage root
    fn root() -> Slot<()>;

    /// Because requests will be deleted from the requests collection,
    /// maintain a simple counter to guarantee unique IDs
    fn slot_next_request_id() -> Slot<u32> {
        Self::root().field(ApprovalStorageKey::NextRequestId)
    }

    /// Approval context included in relevant approval-related calls
    fn slot_config() -> Slot<C> {
        Self::root().field(ApprovalStorageKey::Config)
    }

    /// Reads config from storage. Panics if the component has not been
    /// initialized.
    fn get_config() -> C {
        Self::slot_config()
            .read()
            .unwrap_or_else(|| env::panic_str(NOT_INITIALIZED))
    }

    /// Current list of pending action requests.
    fn slot_request(request_id: u32) -> Slot<ActionRequest<A, S>> {
        Self::root().field(ApprovalStorageKey::Request(request_id))
    }

    /// Get a request by ID
    fn get_request(request_id: u32) -> Option<ActionRequest<A, S>> {
        Self::slot_request(request_id).read()
    }

    /// Must be called before using the Approval construct. Can only be called
    /// once.
    fn init(config: C) {
        require!(
            Self::slot_config().swap(&config).is_none(),
            ALREADY_INITIALIZED,
        );
    }

    /// Creates a new action request initialized with the given approval state
    fn create_request(
        &mut self,
        action: A,
        approval_state: S,
    ) -> Result<u32, CreationError<C::AuthorizationError>> {
        let request_id = Self::slot_next_request_id().read().unwrap_or(0);

        let request = ActionRequest {
            action,
            approval_state,
        };

        let config = Self::get_config();
        let predecessor = env::predecessor_account_id();

        config
            .is_account_authorized(&predecessor, &request)
            .map_err(|source| UnauthorizedAccountError {
                account_id: predecessor,
                source,
            })?;

        Self::slot_next_request_id().write(&(request_id + 1));
        Self::slot_request(request_id).write(&request);

        Ok(request_id)
    }

    /// Executes an action request and removes it from the collection if the
    /// approval state of the request is fulfilled.
    fn execute_request(
        &mut self,
        request_id: u32,
    ) -> Result<A::Output, ExecutionError<C::AuthorizationError, C::ExecutionEligibilityError>>
    {
        Self::is_approved_for_execution(request_id)
            .map_err(|source| ExecutionError::ExecutionEligibility { source })?;

        let predecessor = env::predecessor_account_id();
        let config = Self::get_config();

        let mut request_slot = Self::slot_request(request_id);
        let request = request_slot.read().unwrap();

        config
            .is_account_authorized(&predecessor, &request)
            .map_err(|source| UnauthorizedAccountError {
                account_id: predecessor,
                source,
            })?;

        let result = request.action.execute(self);
        request_slot.remove();

        Ok(result)
    }

    /// Is the given request ID able to be executed if such a request were to
    /// be initiated by an authorized account?
    fn is_approved_for_execution(request_id: u32) -> Result<(), C::ExecutionEligibilityError> {
        let request = Self::slot_request(request_id).read().unwrap();

        let config = Self::get_config();
        config.is_approved_for_execution(&request)
    }

    /// Tries to approve the action request designated by the given request ID
    /// with the given arguments. Panics if the request ID does not exist.
    fn approve_request(
        &mut self,
        request_id: u32,
    ) -> Result<(), ApprovalError<C::AuthorizationError, C::ApprovalError>> {
        let mut request_slot = Self::slot_request(request_id);
        let mut request = request_slot.read().unwrap();

        let predecessor = env::predecessor_account_id();
        let config = Self::get_config();

        config
            .is_account_authorized(&predecessor, &request)
            .map_err(|source| UnauthorizedAccountError {
                account_id: predecessor.clone(),
                source,
            })?;

        config
            .try_approve_with_authorized_account(predecessor, &mut request)
            .map_err(|source| ApprovalError::ApprovalError { source })?;

        request_slot.write(&request);

        Ok(())
    }

    /// Tries to remove the action request indicated by request_id.
    fn remove_request(
        &mut self,
        request_id: u32,
    ) -> Result<(), RemovalError<C::AuthorizationError, C::RemovalError>> {
        let mut request_slot = Self::slot_request(request_id);
        let request = request_slot.read().unwrap();
        let predecessor = env::predecessor_account_id();

        let config = Self::get_config();

        config
            .is_removable(&request)
            .map_err(|source| RemovalError::RemovalNotAllowed { source })?;

        config
            .is_account_authorized(&predecessor, &request)
            .map_err(|source| UnauthorizedAccountError {
                account_id: predecessor,
                source,
            })?;

        request_slot.remove();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::{format, string::ToString, vec::Vec};
    use core::convert::Infallible;

    use near_contract_tools_macros::Rbac;
    use near_sdk::{
        borsh::{self, BorshDeserialize, BorshSerialize},
        near_bindgen,
        test_utils::VMContextBuilder,
        testing_env, AccountId, BorshStorageKey,
    };
    use serde::Serialize;
    use snafu::{whatever, Whatever};

    use crate::{rbac::Rbac, slot::Slot};

    use super::{Action, ActionRequest, ApprovalConfiguration, ApprovalManager};

    #[derive(BorshSerialize, BorshStorageKey)]
    enum Role {
        Multisig,
    }

    #[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone)]
    enum MyAction {
        SayHello,
        SayGoodbye,
    }

    impl Action<Contract> for MyAction {
        type Output = &'static str;

        fn execute(self, _contract: &mut Contract) -> Self::Output {
            match self {
                Self::SayHello => "hello",
                Self::SayGoodbye => "goodbye",
            }
        }
    }

    #[derive(Rbac)]
    #[rbac(roles = "Role", crate = "crate")]
    #[near_bindgen]
    struct Contract {}

    #[near_bindgen]
    impl Contract {
        #[init]
        pub fn new(threshold: u8) -> Self {
            let contract = Self {};

            <Self as ApprovalManager<_, _, _>>::init(MultisigConfig { threshold });

            contract
        }
    }

    impl ApprovalManager<MyAction, MultisigApprovalState, MultisigConfig> for Contract {
        fn root() -> Slot<()> {
            Slot::new(b"a")
        }
    }

    #[derive(BorshSerialize, BorshDeserialize, Debug)]
    struct MultisigConfig {
        pub threshold: u8,
    }

    #[derive(BorshSerialize, BorshDeserialize, Serialize, Default, Debug)]
    struct MultisigApprovalState {
        pub approved_by: Vec<AccountId>,
    }

    impl ApprovalConfiguration<MyAction, MultisigApprovalState> for MultisigConfig {
        type ApprovalError = Whatever;
        type RemovalError = Infallible;
        type AuthorizationError = Whatever;
        type ExecutionEligibilityError = Whatever;

        fn is_approved_for_execution(
            &self,
            action_request: &super::ActionRequest<MyAction, MultisigApprovalState>,
        ) -> Result<(), Self::ExecutionEligibilityError> {
            let valid_signatures = action_request
                .approval_state
                .approved_by
                .iter()
                .filter(|account_id| Contract::has_role(account_id, &Role::Multisig))
                .count();

            let threshold = self.threshold as usize;

            if valid_signatures >= threshold {
                Ok(())
            } else {
                whatever!("Insufficient signatures")
            }
        }

        fn is_removable(
            &self,
            _action_request: &super::ActionRequest<MyAction, MultisigApprovalState>,
        ) -> Result<(), Self::RemovalError> {
            Ok(())
        }

        fn is_account_authorized(
            &self,
            account_id: &AccountId,
            _action_request: &ActionRequest<MyAction, MultisigApprovalState>,
        ) -> Result<(), Self::AuthorizationError> {
            if Contract::has_role(account_id, &Role::Multisig) {
                Ok(())
            } else {
                whatever!("Account is missing Multisig role")
            }
        }

        fn try_approve_with_authorized_account(
            &self,
            account_id: AccountId,
            action_request: &mut ActionRequest<MyAction, MultisigApprovalState>,
        ) -> Result<(), Self::ApprovalError> {
            if action_request
                .approval_state
                .approved_by
                .contains(&account_id)
            {
                whatever!("Already approved by account");
            }

            action_request.approval_state.approved_by.push(account_id);

            Ok(())
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

        let mut contract = Contract::new(2);

        contract.add_role(&alice, &Role::Multisig);
        contract.add_role(&bob, &Role::Multisig);
        contract.add_role(&charlie, &Role::Multisig);

        predecessor(&alice);
        let request_id = contract
            .create_request(MyAction::SayHello, Default::default())
            .unwrap();

        assert_eq!(request_id, 0);
        assert!(Contract::is_approved_for_execution(request_id).is_err());

        contract.approve_request(request_id).unwrap();

        assert!(Contract::is_approved_for_execution(request_id).is_err());

        predecessor(&charlie);
        contract.approve_request(request_id).unwrap();

        assert!(Contract::is_approved_for_execution(request_id).is_ok());

        assert_eq!(contract.execute_request(request_id).unwrap(), "hello");
    }

    #[test]
    #[should_panic(expected = "Already approved by account")]
    fn duplicate_approval() {
        let alice: AccountId = "alice".parse().unwrap();

        let mut contract = Contract::new(2);

        contract.add_role(&alice, &Role::Multisig);

        predecessor(&alice);
        let request_id = contract
            .create_request(MyAction::SayHello, Default::default())
            .unwrap();

        contract.approve_request(request_id).unwrap();

        contract.approve_request(request_id).unwrap();
    }

    #[test]
    #[should_panic = "Insufficient signatures"]
    fn no_execution_before_approval() {
        let alice: AccountId = "alice".parse().unwrap();

        let mut contract = Contract::new(2);

        contract.add_role(&alice, &Role::Multisig);

        predecessor(&alice);

        let request_id = contract
            .create_request(MyAction::SayHello, Default::default())
            .unwrap();

        contract.approve_request(request_id).unwrap();

        contract.execute_request(request_id).unwrap();
    }

    #[test]
    fn successful_removal() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob_acct".parse().unwrap();

        let mut contract = Contract::new(2);

        contract.add_role(&alice, &Role::Multisig);
        contract.add_role(&bob, &Role::Multisig);

        predecessor(&alice);

        let request_id = contract
            .create_request(MyAction::SayHello, Default::default())
            .unwrap();

        contract.approve_request(request_id).unwrap();

        predecessor(&bob);

        contract.remove_request(request_id).unwrap();
    }

    #[test]
    fn dynamic_eligibility() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob_acct".parse().unwrap();
        let charlie: AccountId = "charlie".parse().unwrap();

        let mut contract = Contract::new(2);

        contract.add_role(&alice, &Role::Multisig);
        contract.add_role(&bob, &Role::Multisig);
        contract.add_role(&charlie, &Role::Multisig);

        predecessor(&alice);
        let request_id = contract
            .create_request(MyAction::SayGoodbye, Default::default())
            .unwrap();

        contract.approve_request(request_id).unwrap();

        predecessor(&bob);
        contract.approve_request(request_id).unwrap();

        assert!(Contract::is_approved_for_execution(request_id).is_ok());

        contract.remove_role(&alice, &Role::Multisig);

        assert!(Contract::is_approved_for_execution(request_id).is_err());

        predecessor(&charlie);
        contract.approve_request(request_id).unwrap();

        assert!(Contract::is_approved_for_execution(request_id).is_ok());
    }
}
