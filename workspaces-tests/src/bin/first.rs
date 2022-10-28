#![allow(missing_docs)]
pub fn main() {}

use near_contract_tools::{
    approval::{self, ApprovalManager},
    rbac::Rbac,
    Rbac, SimpleMultisig,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::Base64VecU8,
    near_bindgen,
    serde::{Deserialize, Serialize},
    BorshStorageKey, Gas, GasWeight, PanicOnDefault, Promise,
};

#[derive(BorshStorageKey, BorshSerialize, Debug, Clone)]
pub enum Role {
    Multisig,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum ContractAction {
    Upgrade { code: Base64VecU8 },
}

impl approval::Action<Contract> for ContractAction {
    type Output = Promise;

    fn execute(self, _contract: &mut Contract) -> Self::Output {
        match self {
            ContractAction::Upgrade { code } => Upgrade::new(code.into()).run(),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Debug, Clone, Rbac, SimpleMultisig)]
#[rbac(roles = "Role")]
#[simple_multisig(role = "Role::Multisig", action = "ContractAction")]
#[near_bindgen]
pub struct Contract {}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        <Self as ApprovalManager<_, _, _>>::init(approval::simple_multisig::Configuration::new(
            1, 0,
        ));

        let mut contract = Self {};

        contract.add_role(&env::predecessor_account_id(), &Role::Multisig);

        contract
    }

    pub fn request(&mut self, request: ContractAction) -> u32 {
        self.create_request(request, Default::default()).unwrap()
    }

    pub fn approve(&mut self, request_id: u32) {
        self.approve_request(request_id).unwrap()
    }

    pub fn execute(&mut self, request_id: u32) -> Promise {
        env::log_str("executing request");
        self.execute_request(request_id).unwrap()
    }
}

pub struct Upgrade {
    pub code: Vec<u8>,
    pub function_name: String,
    pub args: Vec<u8>,
    pub minimum_gas: Gas,
}

impl Upgrade {
    pub fn new(code: Vec<u8>) -> Self {
        Self {
            code,
            function_name: "migrate".to_string(),
            args: vec![],
            minimum_gas: Gas(15_000_000_000_000),
        }
    }

    pub fn then(self, function_name: String, args: Vec<u8>) -> Self {
        Self {
            function_name,
            args,
            ..self
        }
    }

    pub fn run(self) -> Promise {
        env::log_str("creating promise");
        Promise::new(env::current_account_id())
            .deploy_contract(self.code)
            .function_call_weight(
                self.function_name,
                self.args,
                0,
                self.minimum_gas,
                GasWeight(u64::MAX),
            )
    }
}
