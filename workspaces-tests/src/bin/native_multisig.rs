// Ignore
pub fn main() {}

use near_contract_tools::{
    approval::{
        native_transaction_action::{self, NativeTransactionAction},
        simple_multisig::Configuration,
        ApprovalManager,
    },
    rbac::Rbac,
    Rbac, SimpleMultisig,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault, Promise,
};

#[derive(BorshSerialize, BorshStorageKey)]
enum Role {
    Multisig,
}

#[derive(PanicOnDefault, BorshSerialize, BorshDeserialize, Rbac, SimpleMultisig)]
#[simple_multisig(action = "NativeTransactionAction", role = "Role::Multisig")]
#[rbac(roles = "Role")]
#[near_bindgen]
pub struct Contract {}

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

    pub fn request(
        &mut self,
        receiver_id: AccountId,
        actions: Vec<native_transaction_action::PromiseAction>,
    ) -> u32 {
        self.require_role(&Role::Multisig);

        let request_id = self.add_request(native_transaction_action::NativeTransactionAction {
            receiver_id,
            actions,
        });

        near_sdk::log!(format!("Request ID: {request_id}"));

        request_id
    }

    pub fn approve(&mut self, request_id: u32) {
        self.approve_request(request_id, None);
    }

    pub fn is_approved(&self, request_id: u32) -> bool {
        <Contract as ApprovalManager<_, _, _>>::is_approved(request_id)
    }

    pub fn execute(&mut self, request_id: u32) -> Promise {
        self.execute_request(request_id)
    }
}
