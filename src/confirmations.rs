//! Queue and approve actions

use std::collections::HashMap;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    require,
};
use serde::{Deserialize, Serialize};

/// Actions can be executed after they are confirmed
pub trait Action {
    /// Return type of the action. Useful if the action creates a `Promise`, for example.
    type Output;
    /// Perform the action. One time only.
    fn execute(self) -> Self::Output;
}

/// The confirmation state determines whether an action request has achieved
/// sufficient confirmations. For example, multisig confirmation state would
/// keep track of who has approved an action request so far.
pub trait ConfirmationState<C> {
    /// Whether the current state represents full confirmation. Note that this
    /// function is called immediately before attempting to execute an action,
    /// so it is possible for this function to respond to externalities (i.e.
    /// changes to contract state other than calls to confirm or reject)
    fn is_confirmed(&self, config: &C) -> bool;

    /// Try to improve the confirmation state. Additional arguments may be
    /// provided, e.g. from the initiating function call
    fn attempt_confirmation(&mut self, args: Option<String>, config: &C);

    /// Try to worsen the confirmation state. Additional arguments may be
    /// provided, e.g. from the initiating function call
    fn attempt_rejection(&mut self, _args: Option<String>, _config: &C) -> bool {
        false
    }
}

/// An action request is composed of an action that will be executed when the
/// associated confirmation state is satisfied
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug)]
pub struct ActionRequest<A, S> {
    /// The action that will be executed when the confirmation state is
    /// fulfilled
    pub action: A,
    /// The associated confirmation state
    pub confirmation_state: S,
}

/// Collection of action requests that manages their confirmation state and
/// execution
#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Default)]
pub struct Confirmations<A, S, C> {
    /// Because requests will be deleted from the requests collection,
    /// maintain a simple counter to guarantee unique IDs
    pub next_request_id: u32,
    /// Current list of pending action requests.
    pub requests: HashMap<u32, ActionRequest<A, S>>,
    /// Confirmation context included in relevant confirmation-related calls
    pub config: C,
}

impl<A, S, C> Confirmations<A, S, C> {
    /// Creates a new instance of the struct with the given config
    pub fn new(config: C) -> Self {
        Self {
            next_request_id: 0,
            requests: Default::default(),
            config,
        }
    }

    /// Creates a new action request initialized with the given confirmation
    /// state
    pub fn add_request_with_state(&mut self, action: A, confirmation_state: S) -> u32 {
        let request_id = self.next_request_id;
        self.next_request_id += 1;

        self.requests.insert(
            request_id,
            ActionRequest {
                action,
                confirmation_state,
            },
        );

        request_id
    }
}

impl<A, S: Default, C> Confirmations<A, S, C> {
    /// Creates a new action request with the default confirmation state
    pub fn add_request(&mut self, action: A) -> u32 {
        let request_id = self.next_request_id;
        self.next_request_id += 1;

        self.requests.insert(
            request_id,
            ActionRequest {
                action,
                confirmation_state: Default::default(),
            },
        );

        request_id
    }
}

impl<A: Action, S: ConfirmationState<C>, C> Confirmations<A, S, C> {
    /// Executes an action request and removes it from the collection if the
    /// confirmation state of the request is fulfilled. Panics otherwise.
    pub fn attempt_execution(&mut self, request_id: u32) -> A::Output {
        require!(
            self.is_confirmed(request_id),
            "Request must be confirmed before it can be executed",
        );

        self.requests
            .remove(&request_id)
            .map(|request| request.action.execute())
            .unwrap()
    }
}

impl<A, S: ConfirmationState<C>, C> Confirmations<A, S, C> {
    /// Returns `true` if the given request ID exists and is confirmed (that
    /// is, the action request may be executed), `false` otherwise.
    pub fn is_confirmed(&self, request_id: u32) -> bool {
        self.requests
            .get(&request_id)
            .map(|request| request.confirmation_state.is_confirmed(&self.config))
            .unwrap_or(false)
    }

    /// Tries to confirm the action request designated by the given request ID
    /// with the given arguments. Panics if the request ID does not exist.
    pub fn attempt_confirmation(&mut self, request_id: u32, args: Option<String>) {
        self.requests
            .get_mut(&request_id)
            .unwrap()
            .confirmation_state
            .attempt_confirmation(args, &self.config);
    }

    /// Tries to reject the action request designated by the given request ID
    /// with the given arguments. Panics if the request ID does not exist.
    pub fn attempt_rejection(&mut self, request_id: u32, args: Option<String>) {
        self.requests
            .get_mut(&request_id)
            .unwrap()
            .confirmation_state
            .attempt_rejection(args, &self.config)
            .then(|| {
                self.requests.remove(&request_id);
            });
    }
}

#[cfg(test)]
mod tests {
    use near_contract_tools_macros::Rbac;
    use near_sdk::{
        borsh::{self, BorshSerialize},
        env, near_bindgen, require,
        test_utils::VMContextBuilder,
        testing_env, AccountId, BorshStorageKey,
    };

    use crate::near_contract_tools;
    use crate::{confirmations::ConfirmationState, rbac::Rbac};

    use super::{Action, Confirmations};

    #[derive(BorshSerialize, BorshStorageKey)]
    enum Role {
        Multisig,
    }

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
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
    struct Contract {
        pub confirmations: Confirmations<MyAction, MultisigConfirmationState, MultisigConfig>,
    }

    #[derive(Debug)]
    struct MultisigConfig {
        pub threshold: u8,
    }

    #[derive(Default, Debug)]
    struct MultisigConfirmationState {
        pub confirmed_by: Vec<AccountId>,
    }

    impl ConfirmationState<MultisigConfig> for MultisigConfirmationState {
        fn is_confirmed(&self, config: &MultisigConfig) -> bool {
            self.confirmed_by
                .iter()
                .filter(|account| {
                    // in case a signatory's role was revoked in the meantime
                    Contract::has_role(&account, &Role::Multisig)
                })
                .count()
                >= config.threshold as usize
        }

        fn attempt_confirmation(&mut self, _args: Option<String>, _config: &MultisigConfig) {
            let predecessor = env::predecessor_account_id();
            require!(
                Contract::has_role(&predecessor, &Role::Multisig),
                "Must have multisig role",
            );
            require!(
                !self.confirmed_by.contains(&predecessor),
                "Already confirmed by this account",
            );

            self.confirmed_by.push(predecessor);
        }

        fn attempt_rejection(&mut self, _args: Option<String>, _config: &MultisigConfig) -> bool {
            let predecessor = env::predecessor_account_id();
            require!(
                Contract::has_role(&predecessor, &Role::Multisig),
                "Must have multisig role",
            );

            self.confirmed_by
                .retain(|signatory| signatory != &predecessor);

            self.confirmed_by.len() == 0
        }
    }

    fn predecessor(account_id: &AccountId) {
        let mut context = VMContextBuilder::new();
        context.predecessor_account_id(account_id.clone());
        testing_env!(context.build());
    }

    #[test]
    fn successful_confirmation() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob_acct".parse().unwrap();
        let charlie: AccountId = "charlie".parse().unwrap();

        let mut contract = Contract {
            confirmations: Confirmations::new(MultisigConfig { threshold: 2 }),
        };

        contract.add_role(&alice, &Role::Multisig);
        contract.add_role(&bob, &Role::Multisig);
        contract.add_role(&charlie, &Role::Multisig);

        let request_id = contract.confirmations.add_request(MyAction::SayHello);

        assert_eq!(request_id, 0);
        assert!(!contract.confirmations.is_confirmed(request_id));

        predecessor(&alice);
        contract
            .confirmations
            .attempt_confirmation(request_id, None);

        assert!(!contract.confirmations.is_confirmed(request_id));

        predecessor(&charlie);
        contract
            .confirmations
            .attempt_confirmation(request_id, None);

        assert!(contract.confirmations.is_confirmed(request_id));

        assert_eq!(
            contract.confirmations.attempt_execution(request_id),
            "hello",
        );
    }

    #[test]
    #[should_panic(expected = "Already confirmed by this account")]
    fn duplicate_confirmation() {
        let alice: AccountId = "alice".parse().unwrap();

        let mut contract = Contract {
            confirmations: Confirmations::new(MultisigConfig { threshold: 2 }),
        };

        contract.add_role(&alice, &Role::Multisig);

        let request_id = contract.confirmations.add_request(MyAction::SayHello);

        predecessor(&alice);
        contract
            .confirmations
            .attempt_confirmation(request_id, None);

        contract
            .confirmations
            .attempt_confirmation(request_id, None);
    }

    #[test]
    #[should_panic = "Request must be confirmed before it can be executed"]
    fn no_execution_before_confirmation() {
        let alice: AccountId = "alice".parse().unwrap();

        let mut contract = Contract {
            confirmations: Confirmations::new(MultisigConfig { threshold: 2 }),
        };

        contract.add_role(&alice, &Role::Multisig);

        let request_id = contract.confirmations.add_request(MyAction::SayHello);

        predecessor(&alice);
        contract
            .confirmations
            .attempt_confirmation(request_id, None);

        contract.confirmations.attempt_execution(request_id);
    }

    #[test]
    fn dynamic_confirmation_calculation() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob_acct".parse().unwrap();
        let charlie: AccountId = "charlie".parse().unwrap();

        let mut contract = Contract {
            confirmations: Confirmations::new(MultisigConfig { threshold: 2 }),
        };

        contract.add_role(&alice, &Role::Multisig);
        contract.add_role(&bob, &Role::Multisig);
        contract.add_role(&charlie, &Role::Multisig);

        let request_id = contract.confirmations.add_request(MyAction::SayGoodbye);

        predecessor(&alice);
        contract
            .confirmations
            .attempt_confirmation(request_id, None);

        predecessor(&bob);
        contract
            .confirmations
            .attempt_confirmation(request_id, None);

        assert!(contract.confirmations.is_confirmed(request_id));

        contract.remove_role(&alice, &Role::Multisig);

        assert!(!contract.confirmations.is_confirmed(request_id));

        predecessor(&charlie);
        contract
            .confirmations
            .attempt_confirmation(request_id, None);

        assert!(contract.confirmations.is_confirmed(request_id));
    }
}
