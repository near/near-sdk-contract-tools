//! Queue and approve actions

// TODO: Use Result instead of panicking in try_* functions
// TODO: Extract error strings into constants

use std::fmt::Display;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, require, BorshStorageKey,
};
use serde::{Deserialize, Serialize};

use crate::slot::Slot;

pub const NOT_INITIALIZED: &str = "Approval::init must be called before use";
pub const ALREADY_INITIALIZED: &str = "Approval::init can only be called once";

pub mod native_action;
pub mod simple_multisig;

/// Actions can be executed after they are approved
pub trait Action: BorshSerialize + BorshDeserialize {
    /// Return type of the action. Useful if the action creates a `Promise`, for example.
    type Output;
    /// Perform the action. One time only.
    fn execute(self) -> Self::Output;
}

/// The approval state determines whether an action request has achieved
/// sufficient approvals. For example, multisig confirmation state would keep
/// track of who has approved an action request so far.
pub trait Approval<C>: Default + BorshSerialize + BorshDeserialize {
    type Error;

    /// Whether the current state represents full approval. Note that this
    /// function is called immediately before attempting to execute an action,
    /// so it is possible for this function to respond to externalities (i.e.
    /// changes to contract state other than calls to approve)
    fn is_fulfilled(&self, config: &C) -> bool;

    /// Try to improve the approval state. Additional arguments may be
    /// provided, e.g. from the initiating function call
    fn try_approve(&mut self, args: Option<String>, config: &C) -> Result<(), Self::Error>;
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

/// Collection of action requests that manages their approval state and
/// execution
pub trait ApprovalManager<A, S, C>
where
    A: Action,
    S: Approval<C> + Serialize,
    C: BorshDeserialize + BorshSerialize,
    S::Error: Display,
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
    fn config() -> C {
        Self::slot_config()
            .read()
            .unwrap_or_else(|| env::panic_str(NOT_INITIALIZED))
    }

    /// Current list of pending action requests.
    fn slot_request(request_id: u32) -> Slot<ActionRequest<A, S>> {
        Self::root().field(ApprovalStorageKey::Request(request_id))
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
    fn add_request_with_state(&mut self, action: A, approval_state: S) -> u32 {
        let request_id = Self::slot_next_request_id().read().unwrap_or(0);
        Self::slot_next_request_id().write(&(request_id + 1));

        Self::slot_request(request_id).write(&ActionRequest {
            action,
            approval_state,
        });

        request_id
    }

    /// Creates a new action request with the default approval state
    fn add_request(&mut self, action: A) -> u32 {
        let request_id = Self::slot_next_request_id().read().unwrap_or(0);
        Self::slot_next_request_id().write(&(request_id + 1));

        Self::slot_request(request_id).write(&ActionRequest {
            action,
            approval_state: Default::default(),
        });

        request_id
    }

    /// Executes an action request and removes it from the collection if the
    /// approval state of the request is fulfilled. Panics otherwise.
    fn try_execute(&mut self, request_id: u32) -> A::Output {
        require!(
            Self::is_approved(request_id),
            "Request must be approved before it can be executed",
        );

        Self::slot_request(request_id)
            .take()
            .map(|request| request.action.execute())
            .unwrap()
    }

    /// Returns `true` if the given request ID exists and is approved (that
    /// is, the action request may be executed), `false` otherwise.
    fn is_approved(request_id: u32) -> bool {
        Self::slot_request(request_id)
            .read()
            .map(|request| {
                let config = Self::config();
                request.approval_state.is_fulfilled(&config)
            })
            .unwrap_or(false)
    }

    /// Tries to approve the action request designated by the given request ID
    /// with the given arguments. Panics if the request ID does not exist.
    fn try_approve(&mut self, request_id: u32, args: Option<String>) {
        let mut request_slot = Self::slot_request(request_id);
        let mut request = request_slot.read().unwrap();

        request
            .approval_state
            .try_approve(args, &Self::config())
            .unwrap_or_else(|e| env::panic_str(&format!("Approval failure: {e}")));

        request_slot.write(&request);
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Display;

    use near_contract_tools_macros::Rbac;
    use near_sdk::{
        borsh::{self, BorshDeserialize, BorshSerialize},
        env, near_bindgen,
        test_utils::VMContextBuilder,
        testing_env, AccountId, BorshStorageKey,
    };
    use serde::Serialize;

    use crate::{approval::Approval, rbac::Rbac};
    use crate::{near_contract_tools, slot::Slot};

    use super::{Action, ApprovalManager};

    #[derive(BorshSerialize, BorshStorageKey)]
    enum Role {
        Multisig,
    }

    #[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq, Clone, Copy)]
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

    enum ApprovalError {
        MissingRole,
        AlreadyApprovedByAccount,
    }

    impl Display for ApprovalError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "{}",
                match self {
                    Self::MissingRole => "Missing required role",
                    Self::AlreadyApprovedByAccount => "Already approved by this account",
                }
            )
        }
    }

    impl Approval<MultisigConfig> for MultisigApprovalState {
        type Error = ApprovalError;

        fn is_fulfilled(&self, config: &MultisigConfig) -> bool {
            self.approved_by
                .iter()
                .filter(|account| {
                    // in case a signatory's role was revoked in the meantime
                    Contract::has_role(&account, &Role::Multisig)
                })
                .count()
                >= config.threshold as usize
        }

        fn try_approve(
            &mut self,
            _args: Option<String>,
            _config: &MultisigConfig,
        ) -> Result<(), Self::Error> {
            let predecessor = env::predecessor_account_id();

            if !Contract::has_role(&predecessor, &Role::Multisig) {
                return Err(ApprovalError::MissingRole);
            }

            if self.approved_by.contains(&predecessor) {
                return Err(ApprovalError::AlreadyApprovedByAccount);
            }

            self.approved_by.push(predecessor);

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

        let request_id = contract.add_request(MyAction::SayHello);

        assert_eq!(request_id, 0);
        assert!(!Contract::is_approved(request_id));

        predecessor(&alice);
        contract.try_approve(request_id, None);

        assert!(!Contract::is_approved(request_id));

        predecessor(&charlie);
        contract.try_approve(request_id, None);

        assert!(Contract::is_approved(request_id));

        assert_eq!(contract.try_execute(request_id), "hello",);
    }

    #[test]
    #[should_panic(expected = "Already approved by this account")]
    fn duplicate_approval() {
        let alice: AccountId = "alice".parse().unwrap();

        let mut contract = Contract::new(2);

        contract.add_role(&alice, &Role::Multisig);

        let request_id = contract.add_request(MyAction::SayHello);

        predecessor(&alice);
        contract.try_approve(request_id, None);

        contract.try_approve(request_id, None);
    }

    #[test]
    #[should_panic = "Request must be approved before it can be executed"]
    fn no_execution_before_approval() {
        let alice: AccountId = "alice".parse().unwrap();

        let mut contract = Contract::new(2);

        contract.add_role(&alice, &Role::Multisig);

        let request_id = contract.add_request(MyAction::SayHello);

        predecessor(&alice);
        contract.try_approve(request_id, None);

        contract.try_execute(request_id);
    }

    #[test]
    fn dynamic_is_approved_calculation() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob_acct".parse().unwrap();
        let charlie: AccountId = "charlie".parse().unwrap();

        let mut contract = Contract::new(2);

        contract.add_role(&alice, &Role::Multisig);
        contract.add_role(&bob, &Role::Multisig);
        contract.add_role(&charlie, &Role::Multisig);

        let request_id = contract.add_request(MyAction::SayGoodbye);

        predecessor(&alice);
        contract.try_approve(request_id, None);

        predecessor(&bob);
        contract.try_approve(request_id, None);

        assert!(Contract::is_approved(request_id));

        contract.remove_role(&alice, &Role::Multisig);

        assert!(!Contract::is_approved(request_id));

        predecessor(&charlie);
        contract.try_approve(request_id, None);

        assert!(Contract::is_approved(request_id));
    }
}
