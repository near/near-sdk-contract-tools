#![allow(missing_docs)]

workspaces_tests::near_sdk!();
compat_use_borsh!();
use near_sdk::{env, near_bindgen, serde::Serialize, BorshStorageKey, PanicOnDefault};
use near_sdk_contract_tools::{
    approval::{simple_multisig::Configuration, *},
    compat_derive_borsh, compat_derive_serde_borsh, compat_derive_storage_key, compat_use_borsh,
    rbac::Rbac,
    Rbac, SimpleMultisig,
};
use std::string::ToString;
use strum_macros::Display;

compat_derive_storage_key! {
    #[derive(Clone, Debug, Display)]
    pub enum Role {
        Member,
    }
}

compat_derive_serde_borsh! {[Serialize, BorshSerialize, BorshDeserialize],
    pub enum CounterAction {
        Increment,
        Decrement,
        Reset,
    }
}

impl Action<Contract> for CounterAction {
    type Output = u32;

    fn execute(self, contract: &mut Contract) -> Self::Output {
        match self {
            CounterAction::Increment => {
                contract.counter += 1;
            }
            CounterAction::Decrement => {
                contract.counter -= 1;
            }
            CounterAction::Reset => {
                contract.counter = 0;
            }
        }

        contract.counter
    }
}

compat_derive_borsh! {
    #[derive(PanicOnDefault, Rbac, SimpleMultisig)]
    #[simple_multisig(action = "CounterAction", role = "Role::Member")]
    #[rbac(roles = "Role")]
    #[near_bindgen]
    pub struct Contract {
        pub counter: u32,
    }
}

#[near_bindgen]
impl Contract {
    const THRESHOLD: u8 = 2;
    const VALIDITY_PERIOD_NANOSECONDS: u64 = 1_000_000 * 1_000 * 60 * 60 * 24 * 7;

    #[init]
    pub fn new() -> Self {
        <Self as ApprovalManager<_, _, _>>::init(Configuration::new(
            Self::THRESHOLD,
            Self::VALIDITY_PERIOD_NANOSECONDS,
        ));

        Self { counter: 0 }
    }

    pub fn obtain_multisig_permission(&mut self) {
        self.add_role(&env::predecessor_account_id(), &Role::Member);
    }

    pub fn request_increment(&mut self) -> u32 {
        self.create_request(CounterAction::Increment, Default::default())
            .map_err(|e| env::panic_str(&e.to_string()))
            .unwrap()
    }

    pub fn request_decrement(&mut self) -> u32 {
        self.create_request(CounterAction::Decrement, Default::default())
            .map_err(|e| env::panic_str(&e.to_string()))
            .unwrap()
    }

    pub fn request_reset(&mut self) -> u32 {
        self.create_request(CounterAction::Reset, Default::default())
            .map_err(|e| env::panic_str(&e.to_string()))
            .unwrap()
    }

    pub fn approve(&mut self, request_id: u32) {
        self.approve_request(request_id)
            .map_err(|e| env::panic_str(&e.to_string()))
            .unwrap()
    }

    pub fn get_request(
        &self,
        request_id: u32,
    ) -> Option<ActionRequest<CounterAction, simple_multisig::ApprovalState>> {
        <Self as ApprovalManager<_, _, _>>::get_request(request_id)
    }

    pub fn is_approved(&self, request_id: u32) -> bool {
        <Self as ApprovalManager<_, _, _>>::is_approved_for_execution(request_id).is_ok()
    }

    pub fn execute(&mut self, request_id: u32) -> u32 {
        self.execute_request(request_id)
            .map_err(|e| env::panic_str(&e.to_string()))
            .unwrap()
    }

    pub fn get_counter(&self) -> u32 {
        self.counter
    }
}

pub fn main() {} // Ignore
