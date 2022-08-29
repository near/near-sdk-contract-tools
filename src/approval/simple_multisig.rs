use std::{borrow::Cow, marker::PhantomData};

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, require, AccountId,
};
use serde::{Deserialize, Serialize};

use crate::approval::ApprovalState;

pub trait SimpleMultisigApprover {
    fn approve(account_id: &AccountId) -> Result<(), Cow<str>>;
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug)]
pub struct SimpleMultisigConfig<A: SimpleMultisigApprover> {
    pub threshold: u8,
    #[borsh_skip]
    #[serde(skip)]
    __approver: PhantomData<A>,
}

impl<A: SimpleMultisigApprover> SimpleMultisigConfig<A> {
    pub fn new(threshold: u8) -> Self {
        Self {
            threshold,
            __approver: PhantomData,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Default, Debug)]
pub struct SimpleMultisigApprovalState {
    pub approved_by: Vec<AccountId>,
}

impl<A: SimpleMultisigApprover> ApprovalState<SimpleMultisigConfig<A>>
    for SimpleMultisigApprovalState
{
    fn is_approved(&self, config: &SimpleMultisigConfig<A>) -> bool {
        self.approved_by
            .iter()
            .filter(|account_id| A::approve(account_id).is_ok())
            .count()
            >= config.threshold as usize
    }

    fn try_approve(&mut self, _args: Option<String>, _config: &SimpleMultisigConfig<A>) {
        let predecessor = env::predecessor_account_id();

        A::approve(&predecessor).unwrap_or_else(|e| env::panic_str(&e));

        require!(
            !self.approved_by.contains(&predecessor),
            "Already approved by this account",
        );

        self.approved_by.push(predecessor);
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

    use crate::{approval::Approval, near_contract_tools, rbac::Rbac, slot::Slot, Rbac};

    use super::{SimpleMultisigApprovalState, SimpleMultisigApprover, SimpleMultisigConfig};

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

    impl Approval<Action, SimpleMultisigApprovalState, SimpleMultisigConfig<Self>> for Contract {
        fn root() -> Slot<()> {
            Slot::new(b"m")
        }
    }

    impl SimpleMultisigApprover for Contract {
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
            <Self as Approval<_, _, _>>::init(SimpleMultisigConfig::new(2));
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
