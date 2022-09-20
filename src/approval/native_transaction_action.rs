//! Approval action type for native NEAR transaction actions (create account,
//! delete account, add key, delete key, deploy contract, function call, stake,
//! transfer)

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    json_types::{U128, U64},
    AccountId, Gas, Promise,
};
use serde::{Deserialize, Serialize};

/// Every native NEAR action can be mapped to a Promise action.
/// NOTE: The native ADD_KEY action is split into two: one for adding a
/// full-access key, one for a function call access key.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum PromiseAction {
    /// Native CREATE_ACCOUNT action
    CreateAccount,
    /// Native DEPLOY_CONTRACT action
    DeployContract {
        /// WASM binary blob
        code: Vec<u8>,
    },
    /// Native FUNCTION_CALL action
    FunctionCall {
        /// Name of function to call on receiver
        function_name: String,
        /// Function input (optional)
        arguments: Vec<u8>,
        /// Attached deposit
        amount: U128,
        /// Attached gas
        gas: U64,
    },
    /// Native TRANSFER action
    Transfer {
        /// Amount of NEAR tokens to transfer to receiver
        amount: U128,
    },
    /// Native STAKE action
    Stake {
        /// Amount of tokens to stake
        amount: U128,
        /// Public key of validator node
        public_key: String,
    },
    /// Native ADD_KEY action for full-access keys
    AddFullAccessKey {
        /// Public key to add to account
        public_key: String,
        /// Starting nonce (default: 0)
        nonce: Option<U64>,
    },
    /// Native ADD_KEY action for function call keys
    AddAccessKey {
        /// Public key to add to account
        public_key: String,
        /// Gas allowance
        allowance: U128,
        /// Target contract account ID
        receiver_id: AccountId,
        /// Restrict this key to calls to these functions
        function_names: Vec<String>,
        /// Starting nonce (default: 0)
        nonce: Option<U64>,
    },
    /// Native DELETE_KEY action
    DeleteKey {
        /// Public key to remove
        public_key: String,
    },
    /// Native DELETE_ACCOUNT action
    DeleteAccount {
        /// Remaining account balance transferred to beneficiary
        beneficiary_id: AccountId,
    },
}

/// A NativeTransactionAction represents a native protocol-level transaction,
/// and is easily (de)serializable into many different formats
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct NativeTransactionAction {
    /// Receiver of the transaction
    pub receiver_id: AccountId,
    /// List of actions to perform on receiver
    pub actions: Vec<PromiseAction>,
}

impl<C> super::Action<C> for NativeTransactionAction {
    type Output = Promise;

    fn execute(self, _contract: &mut C) -> Self::Output {
        let mut promise = Promise::new(self.receiver_id);

        // Construct promise
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
