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
    sys, BorshStorageKey, Gas, PanicOnDefault,
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
    type Output = ();

    fn execute(self, _contract: &mut Contract) -> Self::Output {
        match self {
            ContractAction::Upgrade { code } => upgrade(code.into()),
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
}

#[no_mangle]
pub fn execute() {
    env::setup_panic_hook();

    #[derive(Deserialize)]
    #[serde(crate = "near_sdk::serde")]
    struct Args {
        request_id: u32,
    }

    let Args { request_id } = near_sdk::serde_json::from_slice(&env::input().unwrap()).unwrap();

    let mut contract: Contract = env::state_read().unwrap();
    contract.execute_request(request_id).unwrap();
}

fn upgrade(new_wasm: Vec<u8>) {
    const MIGRATE_METHOD_NAME: &[u8] = b"migrate";
    const UPGRADE_GAS_LEFTOVER: Gas = Gas(5_000_000_000_000);

    unsafe {
        // Load code into register 0 result from the input argument if factory call or from promise if callback.
        sys::input(0);
        // Create a promise batch to upgrade current contract with code from register 0.
        let promise_id = sys::promise_batch_create(
            env::current_account_id().as_bytes().len() as u64,
            env::current_account_id().as_bytes().as_ptr() as u64,
        );
        // Deploy the contract code from register 0.
        sys::promise_batch_action_deploy_contract(
            promise_id,
            new_wasm.len() as u64,
            new_wasm.as_ptr() as u64,
        );
        // Call promise to migrate the state.
        // Batched together to fail upgrade if migration fails.
        sys::promise_batch_action_function_call(
            promise_id,
            MIGRATE_METHOD_NAME.len() as u64,
            MIGRATE_METHOD_NAME.as_ptr() as u64,
            0,
            0,
            0,
            (env::prepaid_gas() - env::used_gas() - UPGRADE_GAS_LEFTOVER).0,
        );
        sys::promise_return(promise_id);
    }
}
