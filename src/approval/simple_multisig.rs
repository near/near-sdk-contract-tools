//! Simple multi-signature wallet component. Generic over approvable actions.
//! Use with NativeTransactionAction for multisig over native transactions.

use std::{fmt::Display, marker::PhantomData};

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, AccountId,
};
use serde::{Deserialize, Serialize};

use crate::approval::Approval;

/// An AccountApprover gatekeeps which accounts are eligible to submit approvals
/// to an ApprovalManager
pub trait AccountApprover {
    /// Error type returned by approve_account on failure (e.g. reason for
    /// ineligibility)
    type Error;

    /// Determines whether an account ID is allowed to submit an approval
    fn approve_account(account_id: &AccountId) -> Result<(), Self::Error>;
}

/// M (threshold) of N approval scheme
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, Debug)]
pub struct Configuration<Ap: AccountApprover> {
    /// How many approvals are required?
    pub threshold: u8,
    #[borsh_skip]
    #[serde(skip)]
    __approver: PhantomData<Ap>,
}

impl<Ap: AccountApprover> Configuration<Ap> {
    /// Create an approval scheme with the given threshold
    pub fn new(threshold: u8) -> Self {
        Self {
            threshold,
            __approver: PhantomData,
        }
    }
}

/// Approval state for simple multisig
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Default, Debug)]
pub struct ApprovalState {
    /// List of accounts that have approved an action thus far
    pub approved_by: Vec<AccountId>,
}

// TODO: use thiserror
/// Why might a simple multisig approval attempt fail?
#[derive(Clone, Debug)]
pub enum ApprovalError<E> {
    /// The account has already approved this action request
    AlreadyApprovedByAccount,
    /// The AccountApprover returned another error
    AccountApproverError(E),
}

impl<E: Display> Display for ApprovalError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyApprovedByAccount => write!(f, "Already approved by this account"),
            Self::AccountApproverError(e) => write!(f, "Approver error: {e}"),
        }
    }
}

impl<Ap> Approval<Configuration<Ap>> for ApprovalState
where
    Ap: AccountApprover,
{
    type Error = ApprovalError<Ap::Error>;

    fn is_fulfilled(&self, config: &Configuration<Ap>) -> bool {
        self.approved_by.len() >= config.threshold as usize
    }

    fn try_approve(
        &mut self,
        _args: Option<String>,
        _config: &Configuration<Ap>,
    ) -> Result<(), Self::Error> {
        let predecessor = env::predecessor_account_id();

        if let Err(e) = Ap::approve_account(&predecessor) {
            return Err(ApprovalError::AccountApproverError(e));
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

    use super::{AccountApprover, ApprovalState, Configuration};

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

    impl AccountApprover for Contract {
        type Error = &'static str;

        fn approve_account(account_id: &near_sdk::AccountId) -> Result<(), Self::Error> {
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
            self.approve_request(request_id, None);
        }

        pub fn execute(&mut self, request_id: u32) -> &'static str {
            self.execute_request(request_id)
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
