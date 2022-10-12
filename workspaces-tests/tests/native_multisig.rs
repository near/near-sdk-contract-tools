#![cfg(not(windows))]

use near_contract_tools::approval::native_transaction_action::PromiseAction;
use near_sdk::{serde_json::json, Gas};
use workspaces::{network::Sandbox, Account, Contract, Worker};

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/native_multisig.wasm");

const SECOND_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/cross_target.wasm");

struct Setup {
    pub worker: Worker<Sandbox>,
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize) -> Setup {
    let worker = workspaces::sandbox().await.unwrap();

    // Initialize contract
    let contract = worker.dev_deploy(&WASM.to_vec()).await.unwrap();
    // contract.as_account()
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

async fn setup_roles(num_accounts: usize) -> Setup {
    let s = setup(num_accounts).await;

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

#[tokio::test]
async fn transfer() {
    let Setup {
        contract, accounts, ..
    } = setup_roles(3).await;

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
            .view(
                "is_approved",
                json!({ "request_id": request_id })
                    .to_string()
                    .as_bytes()
                    .to_vec(),
            )
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
    } = setup_roles(3).await;

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
    } = setup_roles(3).await;

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
        .view("get_value", vec![])
        .await
        .unwrap()
        .json::<String>()
        .unwrap();

    assert_eq!(value_before, "");

    let calls_before = second_contract
        .view("get_calls", vec![])
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
        .view("get_value", vec![])
        .await
        .unwrap()
        .json::<String>()
        .unwrap();

    assert_eq!(value_after, "Hello, world!");

    let calls_after = second_contract
        .view("get_calls", vec![])
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    assert_eq!(calls_after, 1);
}
