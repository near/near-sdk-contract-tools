//! Queue and approve actions

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, require, AccountId, BorshStorageKey,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::slot::Slot;

/// Error message emitted when the component is used before it is initialized
pub const NOT_INITIALIZED: &str = "init must be called before use";
/// Error message emitted when the init function is called multiple times
pub const ALREADY_INITIALIZED: &str = "init can only be called once";

pub mod native_transaction_action;
pub mod simple_multisig;

/// Actions can be executed after they are approved
pub trait Action {
    /// Return type of the action. Useful if the action creates a `Promise`, for example.
    type Output;
    /// Perform the action. One time only.
    fn execute(self) -> Self::Output;
}

/// Defines the operating parameters for an ApprovalManager and performs
/// approvals
pub trait ApprovalConfiguration<A, S> {
    /// Approval errors, e.g. "this account has already approved this request",
    /// "this request is not allowed to be approved yet", etc.
    type Error;

    /// Has the request reached full approval?
    fn is_approved_for_execution(&self, action_request: &ActionRequest<A, S>) -> bool;

    /// Can this request be removed by an allowed account?
    fn is_removable(&self, action_request: &ActionRequest<A, S>) -> bool;

    /// Is the account allowed to execute, approve, or remove this request?
    fn is_account_authorized(
        &self,
        account_id: &AccountId,
        action_request: &ActionRequest<A, S>,
    ) -> bool;

    /// Modify action_request.approval_state in-place to increase approval
    fn try_approve_with_authorized_account(
        &self,
        account_id: AccountId,
        action_request: &mut ActionRequest<A, S>,
    ) -> Result<(), Self::Error>;
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

/// Top-level errors that may occur when attempting to approve a request
#[derive(Error, Debug)]
pub enum ApprovalError<E> {
    /// The account is not allowed to act on requests
    #[error("Unauthorized account")]
    UnauthorizedAccount,
    /// The approval function encountered another error
    #[error("Approval error: {0}")]
    ApprovalError(E),
}

/// Errors that may occur when trying to execute a request
#[derive(Error, Debug)]
pub enum ExecutionError {
    /// The account is not allowed to act on requests
    #[error("Unauthorized account")]
    UnauthorizedAccount,
    /// Unapproved requests cannot be executed
    #[error("Request not approved")]
    RequestNotApproved,
}

/// Errors that may occur when trying to create a request
#[derive(Error, Debug)]
pub enum CreationError {
    /// The account is not allowed to act on requests
    #[error("Unauthorized account")]
    UnauthorizedAccount,
}

/// Errors that may occur when trying to remove a request
#[derive(Error, Debug)]
pub enum RemovalError {
    /// The account is not allowed to act on requests
    #[error("Unauthorized account")]
    UnauthorizedAccount,
    /// This request is not (yet?) allowed to be removed
    #[error("Removal not allowed")]
    RemovalNotAllowed,
}

/// Collection of action requests that manages their approval state and
/// execution
pub trait ApprovalManager<A, S, C>
where
    A: Action + BorshSerialize + BorshDeserialize,
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
    fn create_request(&mut self, action: A, approval_state: S) -> Result<u32, CreationError> {
        let request_id = Self::slot_next_request_id().read().unwrap_or(0);

        let request = ActionRequest {
            action,
            approval_state,
        };

        let config = Self::get_config();
        let predecessor = env::predecessor_account_id();

        if !config.is_account_authorized(&predecessor, &request) {
            return Err(CreationError::UnauthorizedAccount);
        }

        Self::slot_next_request_id().write(&(request_id + 1));
        Self::slot_request(request_id).write(&request);

        Ok(request_id)
    }

    /// Executes an action request and removes it from the collection if the
    /// approval state of the request is fulfilled. Panics otherwise.
    fn execute_request(&mut self, request_id: u32) -> Result<A::Output, ExecutionError> {
        if !Self::is_approved(request_id) {
            return Err(ExecutionError::RequestNotApproved);
        }

        let predecessor = env::predecessor_account_id();
        let config = Self::get_config();

        let mut request_slot = Self::slot_request(request_id);
        let request = request_slot.read().unwrap();

        if !config.is_account_authorized(&predecessor, &request) {
            return Err(ExecutionError::UnauthorizedAccount);
        }

        let result = request.action.execute();
        request_slot.remove();

        Ok(result)
    }

    /// Returns `true` if the given request ID exists and is approved (that
    /// is, the action request may be executed), `false` otherwise.
    fn is_approved(request_id: u32) -> bool {
        Self::slot_request(request_id)
            .read()
            .map(|request| {
                let config = Self::get_config();
                config.is_approved_for_execution(&request)
            })
            .unwrap_or(false)
    }

    /// Tries to approve the action request designated by the given request ID
    /// with the given arguments. Panics if the request ID does not exist.
    fn approve_request(&mut self, request_id: u32) -> Result<(), ApprovalError<C::Error>> {
        let mut request_slot = Self::slot_request(request_id);
        let mut request = request_slot.read().unwrap();

        let predecessor = env::predecessor_account_id();
        let config = Self::get_config();

        if !config.is_account_authorized(&predecessor, &request) {
            return Err(ApprovalError::UnauthorizedAccount);
        }

        config
            .try_approve_with_authorized_account(predecessor, &mut request)
            .map_err(ApprovalError::ApprovalError)?;

        request_slot.write(&request);

        Ok(())
    }

    /// Tries to remove the action request indicated by request_id.
    fn remove_request(&mut self, request_id: u32) -> Result<(), RemovalError> {
        let mut request_slot = Self::slot_request(request_id);
        let request = request_slot.read().unwrap();
        let predecessor = env::predecessor_account_id();

        let config = Self::get_config();

        if !config.is_removable(&request) {
            return Err(RemovalError::RemovalNotAllowed);
        }

        config
            .is_account_authorized(&predecessor, &request)
            .then(|| {
                request_slot.remove();
            })
            .ok_or(RemovalError::UnauthorizedAccount)
    }
}

#[cfg(test)]
mod tests {
    use near_contract_tools_macros::Rbac;
    use near_sdk::{
        borsh::{self, BorshDeserialize, BorshSerialize},
        near_bindgen,
        test_utils::VMContextBuilder,
        testing_env, AccountId, BorshStorageKey,
    };
    use serde::Serialize;
    use thiserror::Error;

    use crate::rbac::Rbac;
    use crate::{near_contract_tools, slot::Slot};

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

    impl Action for MyAction {
        type Output = &'static str;

        fn execute(self) -> Self::Output {
            match self {
                Self::SayHello => {
                    println!("Hello!");
                    "hello"
                }
                Self::SayGoodbye => {
                    println!("Goodbye!");
                    "goodbye"
                }
            }
        }
    }

    #[derive(Rbac)]
    #[rbac(roles = "Role")]
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

    #[derive(Error, Debug)]
    enum ApprovalError {
        #[error("Already approved by this account")]
        AlreadyApprovedByAccount,
    }

    impl ApprovalConfiguration<MyAction, MultisigApprovalState> for MultisigConfig {
        type Error = ApprovalError;

        fn is_approved_for_execution(
            &self,
            action_request: &super::ActionRequest<MyAction, MultisigApprovalState>,
        ) -> bool {
            action_request
                .approval_state
                .approved_by
                .iter()
                .filter(|account_id| Contract::has_role(account_id, &Role::Multisig))
                .count()
                >= self.threshold as usize
        }

        fn is_removable(
            &self,
            _action_request: &super::ActionRequest<MyAction, MultisigApprovalState>,
        ) -> bool {
            true
        }

        fn is_account_authorized(
            &self,
            account_id: &AccountId,
            _action_request: &ActionRequest<MyAction, MultisigApprovalState>,
        ) -> bool {
            Contract::has_role(account_id, &Role::Multisig)
        }

        fn try_approve_with_authorized_account(
            &self,
            account_id: AccountId,
            action_request: &mut ActionRequest<MyAction, MultisigApprovalState>,
        ) -> Result<(), Self::Error> {
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
        assert!(!Contract::is_approved(request_id));

        contract.approve_request(request_id).unwrap();

        assert!(!Contract::is_approved(request_id));

        predecessor(&charlie);
        contract.approve_request(request_id).unwrap();

        assert!(Contract::is_approved(request_id));

        assert_eq!(contract.execute_request(request_id).unwrap(), "hello");
    }

    #[test]
    #[should_panic(expected = "ApprovalError(AlreadyApprovedByAccount)")]
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
    #[should_panic = "RequestNotApproved"]
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

        assert!(Contract::is_approved(request_id));

        contract.remove_role(&alice, &Role::Multisig);

        assert!(!Contract::is_approved(request_id));

        predecessor(&charlie);
        contract.approve_request(request_id).unwrap();

        assert!(Contract::is_approved(request_id));
    }
}
