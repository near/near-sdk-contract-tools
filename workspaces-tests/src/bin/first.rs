#![allow(missing_docs)]
pub fn main() {}
use near_contract_tools::upgrade::{upgrade, Upgrade, UpgradeHook};
const WASM: &[u8] = include_bytes!("../../../target/wasm32-unknown-unknown/release/first.wasm");

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
    type Output = ();

    fn execute(self, _contract: &mut Contract) -> Self::Output {
        match self {
            ContractAction::Upgrade { code } => upgrade::<Contract>(code.into()),
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

    pub fn execute(&mut self, request_id: u32) {
        env::log_str("executing request");
        self.execute_request(request_id).unwrap()
    }
}

impl UpgradeHook for Contract {
    fn on_upgrade() {}
}

impl Upgrade for Contract {
    #[no_mangle]
    fn upgrade_contract() {
        Self::on_upgrade();
        let code = Base64VecU8::from(Vec::from(WASM));
        upgrade::<Contract>(code.try_to_vec().unwrap());
    }
}
