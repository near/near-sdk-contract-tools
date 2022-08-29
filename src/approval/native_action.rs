use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    json_types::{U128, U64},
    AccountId, Gas, Promise,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum PromiseAction {
    CreateAccount,
    DeployContract {
        code: Vec<u8>,
    },
    FunctionCall {
        function_name: String,
        arguments: Vec<u8>,
        amount: U128,
        gas: U64,
    },
    Transfer {
        amount: U128,
    },
    Stake {
        amount: U128,
        public_key: String,
    },
    AddFullAccessKey {
        public_key: String,
        nonce: Option<U64>,
    },
    AddAccessKey {
        public_key: String,
        allowance: U128,
        receiver_id: AccountId,
        function_names: Vec<String>,
        nonce: Option<U64>,
    },
    DeleteKey {
        public_key: String,
    },
    DeleteAccount {
        beneficiary_id: AccountId,
    },
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct NativeAction {
    pub receiver_id: AccountId,
    pub actions: Vec<PromiseAction>,
}

impl super::Action for NativeAction {
    type Output = Promise;

    fn execute(self) -> Self::Output {
        let mut promise = Promise::new(self.receiver_id);

        for action in self.actions {
            promise = match action {
                PromiseAction::AddAccessKey {
                    public_key,
                    allowance,
                    receiver_id,
                    function_names,
                    nonce,
                } => promise.add_access_key_with_nonce(
                    public_key.parse().unwrap(),
                    allowance.into(),
                    receiver_id,
                    function_names.join(","),
                    nonce.map(Into::into).unwrap_or(0),
                ),
                PromiseAction::AddFullAccessKey { public_key, nonce } => promise
                    .add_full_access_key_with_nonce(
                        public_key.parse().unwrap(),
                        nonce.map(Into::into).unwrap_or(0),
                    ),
                PromiseAction::CreateAccount => promise.create_account(),
                PromiseAction::DeployContract { code } => promise.deploy_contract(code),
                PromiseAction::FunctionCall {
                    function_name,
                    arguments,
                    amount,
                    gas,
                } => {
                    promise.function_call(function_name, arguments, amount.into(), Gas(gas.into()))
                }
                PromiseAction::Transfer { amount } => promise.transfer(amount.into()),
                PromiseAction::Stake { amount, public_key } => {
                    promise.stake(amount.into(), public_key.parse().unwrap())
                }
                PromiseAction::DeleteKey { public_key } => {
                    promise.delete_key(public_key.parse().unwrap())
                }
                PromiseAction::DeleteAccount { beneficiary_id } => {
                    promise.delete_account(beneficiary_id)
                }
            };
        }

        promise
    }
}
