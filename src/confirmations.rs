use std::collections::HashMap;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    require,
};
use serde::{Deserialize, Serialize};

pub trait Action {
    type Output;
    fn execute(self) -> Self::Output;
}

pub trait ConfirmationState<C> {
    fn is_confirmed(&self, config: &C) -> bool;
    fn attempt_confirmation(&mut self, args: Option<String>, config: &C);
    fn attempt_rejection(&mut self, _args: Option<String>, _config: &C) -> bool {
        false
    }
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Debug)]
pub struct ActionRequest<A, S> {
    pub action: A,
    pub confirmation_state: S,
}

#[derive(Serialize, Deserialize, BorshSerialize, BorshDeserialize, Default)]
pub struct Confirmations<A, S, C> {
    pub next_request_id: u32,
    pub requests: HashMap<u32, ActionRequest<A, S>>,
    pub config: C,
}

impl<A, S, C> Confirmations<A, S, C> {
    pub fn new(config: C) -> Self {
        Self {
            next_request_id: 0,
            requests: Default::default(),
            config,
        }
    }

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
    pub fn is_confirmed(&self, request_id: u32) -> bool {
        self.requests
            .get(&request_id)
            .map(|request| request.confirmation_state.is_confirmed(&self.config))
            .unwrap_or(false)
    }

    pub fn attempt_confirmation(&mut self, request_id: u32, args: Option<String>) {
        if let Some(ref mut request) = self.requests.get_mut(&request_id) {
            request
                .confirmation_state
                .attempt_confirmation(args, &self.config);
        }
    }

    pub fn attempt_rejection(&mut self, request_id: u32, args: Option<String>) {
        self.requests
            .get_mut(&request_id)
            .map(|request| {
                request
                    .confirmation_state
                    .attempt_rejection(args, &self.config)
            })
            .unwrap_or(false)
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

    use super::{Action, ActionRequest, Confirmations};

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

        fn attempt_confirmation(&mut self, args: Option<String>, config: &MultisigConfig) {
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
