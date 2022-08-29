use std::{fmt::Display, marker::PhantomData};

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, AccountId,
};
use serde::{Deserialize, Serialize};

use crate::approval::Approval;

pub trait Approver {
    type Error;
    fn approve(account_id: &AccountId) -> Result<(), Self::Error>;
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug)]
pub struct Configuration<A: Approver> {
    pub threshold: u8,
    #[borsh_skip]
    #[serde(skip)]
    __approver: PhantomData<A>,
}

impl<A: Approver> Configuration<A> {
    pub fn new(threshold: u8) -> Self {
        Self {
            threshold,
            __approver: PhantomData,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Default, Debug)]
pub struct ApprovalState {
    pub approved_by: Vec<AccountId>,
}

#[derive(Debug)]
pub enum ApprovalError<E> {
    AlreadyApprovedByAccount,
    ApproverError(E),
}

impl<E: Display> Display for ApprovalError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyApprovedByAccount => write!(f, "Already approved by this account"),
            Self::ApproverError(e) => write!(f, "Approver error: {e}"),
        }
    }
}

impl<A: Approver> Approval<Configuration<A>> for ApprovalState {
    type Error = ApprovalError<A::Error>;

    fn is_fulfilled(&self, config: &Configuration<A>) -> bool {
        self.approved_by
            .iter()
            .filter(|account_id| A::approve(account_id).is_ok())
            .count()
            >= config.threshold as usize
    }

    fn try_approve(
        &mut self,
        _args: Option<String>,
        _config: &Configuration<A>,
    ) -> Result<(), Self::Error> {
        let predecessor = env::predecessor_account_id();

        if let Err(e) = A::approve(&predecessor) {
            return Err(ApprovalError::ApproverError(e));
        }

        if self.approved_by.contains(&predecessor) {
            return Err(ApprovalError::AlreadyApprovedByAccount);
        }

        self.approved_by.push(predecessor);

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

    use crate::{approval::ApprovalManager, near_contract_tools, rbac::Rbac, slot::Slot, Rbac};

    use super::{ApprovalState, Approver, Configuration};

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

    impl Approver for Contract {
        type Error = &'static str;

        fn approve(account_id: &near_sdk::AccountId) -> Result<(), Self::Error> {
            if Self::has_role(account_id, &Role::Multisig) {
                Ok(())
            } else {
                Err("Must have multisig role")
            }
        }
    }

    #[near_bindgen]
    impl Contract {
        #[init]
        pub fn new() -> Self {
            <Self as ApprovalManager<_, _, _>>::init(Configuration::new(2));
            Self {}
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

            let request_id = self.add_request(action);

            request_id
        }

        pub fn approve(&mut self, request_id: u32) {
            self.try_approve(request_id, None);
        }

        pub fn execute(&mut self, request_id: u32) -> &'static str {
            self.try_execute(request_id)
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
}
