#![allow(missing_docs)]

workspaces_tests::predicate!();

use near_sdk::{
    env,
    json_types::Base64VecU8,
    near_bindgen,
    serde::{Deserialize, Serialize},
    BorshStorageKey, PanicOnDefault,
};
use near_sdk_contract_tools::{
    approval::{self, ApprovalManager},
    compat_derive_borsh, compat_derive_serde_borsh, compat_derive_storage_key,
    owner::*,
    rbac::Rbac,
    Owner, Rbac, SimpleMultisig, Upgrade,
};

compat_derive_storage_key! {
    #[derive(Debug, Clone)]
    pub enum Role {
        Multisig,
    }
}

compat_derive_serde_borsh! {
    #[derive(Debug, Clone)]
    pub enum ContractAction {
        Upgrade { code: Base64VecU8 },
    }
}

impl approval::Action<Contract> for ContractAction {
    type Output = ();

    fn execute(self, _contract: &mut Contract) -> Self::Output {
        match self {
            ContractAction::Upgrade { code } => _contract.upgrade(code.into()),
        }
    }
}

compat_derive_borsh! {
    #[derive(
        PanicOnDefault,
        Owner,
        Debug,
        Clone,
        Rbac,
        Upgrade,
        SimpleMultisig,
    )]
    #[rbac(roles = "Role")]
    #[simple_multisig(role = "Role::Multisig", action = "ContractAction")]
    #[upgrade(serializer = "borsh", hook = "owner")]
    #[near_bindgen]
    pub struct Contract {
        pub foo: u32,
    }
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        <Self as ApprovalManager<_, _, _>>::init(approval::simple_multisig::Configuration::new(
            1, 0,
        ));

        let mut contract = Self { foo: 0 };

        let predecessor = env::predecessor_account_id();

        Owner::init(&mut contract, &predecessor);

        contract.add_role(&predecessor, &Role::Multisig);

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
