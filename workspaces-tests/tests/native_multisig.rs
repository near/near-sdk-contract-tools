#![cfg(not(windows))]

use std::time::Duration;

use near_crypto::{KeyType, SecretKey};
use near_sdk::{serde_json::json, Gas, ONE_NEAR};
use near_sdk_contract_tools::approval::native_transaction_action::PromiseAction;
use tokio::time::sleep;
use workspaces::{
    result::{ExecutionResult, Value},
    sandbox,
    types::{AccessKeyPermission, Finality},
    Account, Contract, DevNetwork, Worker,
};

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/native_multisig.wasm");

const SECOND_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/cross_target.wasm");

struct Setup<T: DevNetwork> {
    pub worker: Worker<T>,
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

async fn setup<T: DevNetwork>(worker: Worker<T>, num_accounts: usize) -> Setup<T> {
    // Initialize contract
    let contract = worker.dev_deploy(&WASM.to_vec()).await.unwrap();
    contract.call("new").transact().await.unwrap().unwrap();

    // Initialize user accounts
    let mut accounts = vec![];
    for _ in 0..(num_accounts + 1) {
        accounts.push(worker.dev_create_account().await.unwrap());
    }

    Setup {
        worker,
        contract,
        accounts,
    }
}

async fn setup_roles<T: DevNetwork>(worker: Worker<T>, num_accounts: usize) -> Setup<T> {
    let s = setup(worker, num_accounts).await;

    for account in s.accounts[..s.accounts.len() - 1].iter() {
        account
            .call(s.contract.id(), "obtain_multisig_permission")
            .transact()
            .await
            .unwrap()
            .unwrap();
    }

    s
}

async fn double_approve_and_execute(
    contract: &Contract,
    signer_1: &Account,
    signer_2: &Account,
    executor: &Account,
    request_id: u32,
) -> ExecutionResult<Value> {
    signer_1
        .call(contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    signer_2
        .call(contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    executor
        .call(contract.id(), "execute")
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn create_account() {
    let Setup {
        contract,
        accounts,
        worker,
    } = setup_roles(sandbox().await.unwrap(), 2).await;

    let alice = &accounts[0];
    let bob = &accounts[1];

    let new_account_id_str = format!("new.{}", contract.id());
    let new_account_id: workspaces::AccountId = new_account_id_str.parse().unwrap();

    // Account does not exist yet
    assert!(worker.view_account(&new_account_id).await.is_err());

    let request_id = alice
        .call(contract.id(), "request")
        .args_json(json!({
            "receiver_id": new_account_id_str.clone(),
            "actions": [
                PromiseAction::CreateAccount,
                PromiseAction::Transfer { amount: ONE_NEAR.into() }
            ],
        }))
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    double_approve_and_execute(&contract, alice, bob, alice, request_id).await;

    let state = worker.view_account(&new_account_id).await.unwrap();
    assert_eq!(state.balance, ONE_NEAR);
}

#[tokio::test]
async fn add_remove_key() {
    let Setup {
        contract, accounts, ..
    } = setup_roles(sandbox().await.unwrap(), 2).await;

    let alice = &accounts[0];
    let bob = &accounts[1];

    // Add a new access key to the contract account
    let execute_actions = |actions: Vec<PromiseAction>| {
        let contract = &contract;

        async move {
            let request_id = alice
                .call(contract.id(), "request")
                .args_json(json!({
                    "receiver_id": contract.id(),
                    "actions": actions,
                }))
                .transact()
                .await
                .unwrap()
                .json::<u32>()
                .unwrap();

            double_approve_and_execute(contract, alice, bob, alice, request_id).await;

            // Finality is apparently insufficient here, as I was still getting some
            // errors on both Testnet and Sandbox if I didn't add the delay.
            sleep(Duration::from_secs(1)).await;
        }
    };

    // Add full-access key
    let full_access_key = {
        let secret_key = SecretKey::from_random(KeyType::ED25519);
        let new_public_key = secret_key.public_key();
        let new_public_key_string = new_public_key.to_string();

        let keys_before = contract
            .view_access_keys()
            .finality(Finality::Final)
            .await
            .unwrap();

        // workspaces::types::PublicKey wrapper type's contents are package-private
        // and there is no Display/.to_string() implementation.
        let new_key_json_string = near_sdk::serde_json::to_string(&new_public_key_string).unwrap();

        assert!(
            keys_before
                .iter()
                .find(|a| near_sdk::serde_json::to_string(&a.public_key).unwrap()
                    == new_key_json_string)
                .is_none(),
            "New key does not exist in access keys before being added"
        );

        execute_actions(vec![PromiseAction::AddFullAccessKey {
            public_key: new_public_key_string.clone(),
            nonce: None,
        }])
        .await;

        let keys_after = contract
            .view_access_keys()
            .finality(Finality::Final)
            .await
            .unwrap();

        assert_eq!(
            keys_before.len() + 1,
            keys_after.len(),
            "There should be exactly one additional access key"
        );

        let key = keys_after
            .iter()
            .find(|a| {
                near_sdk::serde_json::to_string(&a.public_key).unwrap() == new_key_json_string
            })
            .unwrap();

        match &key.access_key.permission {
            AccessKeyPermission::FullAccess => {}
            _ => panic!("Expected full access key"),
        }

        new_public_key_string
    };

    // Add function-call access key
    let function_call_key = {
        let secret_key = SecretKey::from_random(KeyType::ED25519);
        let new_public_key = secret_key.public_key();
        let new_public_key_string = new_public_key.to_string();

        let keys_before = contract
            .view_access_keys()
            .finality(Finality::Final)
            .await
            .unwrap();

        // workspaces::types::PublicKey wrapper type's contents are package-private
        // and there is no Display/.to_string() implementation.
        let new_key_json_string = near_sdk::serde_json::to_string(&new_public_key_string).unwrap();

        assert!(
            keys_before
                .iter()
                .find(|a| near_sdk::serde_json::to_string(&a.public_key).unwrap()
                    == new_key_json_string)
                .is_none(),
            "New key does not exist in access keys before being added"
        );

        execute_actions(vec![PromiseAction::AddAccessKey {
            public_key: new_public_key_string.clone(),
            allowance: (1234567890).into(),
            receiver_id: alice.id().parse().unwrap(),
            function_names: vec!["one".into(), "two".into(), "three".into()],
            nonce: None,
        }])
        .await;

        let keys_after = contract
            .view_access_keys()
            .finality(Finality::Final)
            .await
            .unwrap();

        assert_eq!(
            keys_before.len() + 1,
            keys_after.len(),
            "There should be exactly one additional access key"
        );

        let key = keys_after
            .iter()
            .find(|a| {
                near_sdk::serde_json::to_string(&a.public_key).unwrap() == new_key_json_string
            })
            .unwrap();

        let perm = match &key.access_key.permission {
            AccessKeyPermission::FunctionCall(fc) => fc,
            _ => panic!("Expected function call permission"),
        };

        assert_eq!(perm.allowance, Some(1234567890));
        assert_eq!(perm.method_names, &["one", "two", "three"]);
        assert_eq!(perm.receiver_id, alice.id().to_string());

        new_public_key_string
    };

    // Delete the access keys
    {
        let keys_before = contract
            .view_access_keys()
            .finality(Finality::Final)
            .await
            .unwrap();

        execute_actions(vec![
            PromiseAction::DeleteKey {
                public_key: full_access_key.clone(),
            },
            PromiseAction::DeleteKey {
                public_key: function_call_key.clone(),
            },
        ])
        .await;

        let keys_after = contract
            .view_access_keys()
            .finality(Finality::Final)
            .await
            .unwrap();

        assert_eq!(
            keys_before.len() - 2,
            keys_after.len(),
            "There should be exactly two fewer access keys"
        );

        let full_json = near_sdk::serde_json::to_string(&full_access_key).unwrap();
        let func_json = near_sdk::serde_json::to_string(&function_call_key).unwrap();

        assert!(keys_after
            .iter()
            .find(|a| {
                let k = near_sdk::serde_json::to_string(&a.public_key).unwrap();
                k == full_json || k == func_json
            })
            .is_none());
    }
}

#[tokio::test]
async fn transfer() {
    let Setup {
        contract, accounts, ..
    } = setup_roles(sandbox().await.unwrap(), 3).await;

    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    // Send 10 NEAR to charlie
    let request_id = alice
        .call(contract.id(), "request")
        .args_json(json!({
            "receiver_id": charlie.id(),
            "actions": [
                PromiseAction::Transfer {
                    amount: (near_sdk::ONE_NEAR * 10).into(),
                },
            ],
        }))
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    let is_approved = || async {
        contract
            .view("is_approved")
            .args_json(json!({ "request_id": request_id }))
            .await
            .unwrap()
            .json::<bool>()
            .unwrap()
    };

    assert!(!is_approved().await);

    alice
        .call(contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert!(!is_approved().await);

    bob.call(contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert!(is_approved().await);

    charlie
        .call(contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert!(is_approved().await);

    let balance_before = charlie.view_account().await.unwrap().balance;

    alice
        .call(contract.id(), "execute")
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let balance_after = charlie.view_account().await.unwrap().balance;

    // charlie's balance should have increased by exactly 10 NEAR
    assert_eq!(balance_after - balance_before, near_sdk::ONE_NEAR * 10);
}

#[tokio::test]
async fn reflexive_xcc() {
    let Setup {
        contract, accounts, ..
    } = setup_roles(sandbox().await.unwrap(), 3).await;

    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    let actions = vec![PromiseAction::FunctionCall {
        function_name: "private_add_one".into(),
        arguments: json!({ "value": 25 }).to_string().as_bytes().to_vec(),
        amount: 0.into(),
        gas: (Gas::ONE_TERA.0 * 50).into(),
    }];

    let request_id = alice
        .call(contract.id(), "request")
        .args_json(json!({
            "receiver_id": contract.id(),
            "actions": actions,
        }))
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    alice
        .call(contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    bob.call(contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = charlie
        .call(contract.id(), "execute")
        .max_gas()
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    assert_eq!(result, 26);
}

#[tokio::test]
async fn external_xcc() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup_roles(sandbox().await.unwrap(), 3).await;

    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    let second_contract = worker.dev_deploy(&SECOND_WASM.to_vec()).await.unwrap();
    second_contract
        .call("new")
        .args_json(json!({ "owner_id": contract.id() }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let actions = vec![PromiseAction::FunctionCall {
        function_name: "set_value".into(),
        arguments: json!({ "value": "Hello, world!" })
            .to_string()
            .as_bytes()
            .to_vec(),
        amount: 0.into(),
        gas: (Gas::ONE_TERA.0 * 50).into(),
    }];

    let request_id = alice
        .call(contract.id(), "request")
        .args_json(json!({
            "receiver_id": second_contract.id(),
            "actions": actions,
        }))
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    alice
        .call(contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    bob.call(contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let value_before = second_contract
        .view("get_value")
        .await
        .unwrap()
        .json::<String>()
        .unwrap();

    assert_eq!(value_before, "");

    let calls_before = second_contract
        .view("get_calls")
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    assert_eq!(calls_before, 0);

    charlie
        .call(contract.id(), "execute")
        .max_gas()
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let value_after = second_contract
        .view("get_value")
        .await
        .unwrap()
        .json::<String>()
        .unwrap();

    assert_eq!(value_after, "Hello, world!");

    let calls_after = second_contract
        .view("get_calls")
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    assert_eq!(calls_after, 1);
}
