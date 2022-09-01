#![cfg(not(windows))]

use near_contract_tools::approval::native_transaction_action::PromiseAction;
use near_sdk::{serde_json::json, Gas};
use workspaces::{network::Sandbox, prelude::*, Account, Contract, Network, Worker};

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/native_multisig.wasm");

struct Setup<N: Network> {
    pub worker: Worker<N>,
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize) -> Setup<Sandbox> {
    let worker = workspaces::sandbox().await.unwrap();

    // Initialize contract
    let contract = worker.dev_deploy(&WASM.to_vec()).await.unwrap();
    contract.call(&worker, "new").transact().await.unwrap();

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

async fn setup_roles(num_accounts: usize) -> Setup<impl Network> {
    let s = setup(num_accounts).await;

    for account in s.accounts[..s.accounts.len() - 1].iter() {
        account
            .call(&s.worker, s.contract.id(), "obtain_multisig_permission")
            .transact()
            .await
            .unwrap();
    }

    s
}

#[tokio::test]
#[ignore]
async fn transfer() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup_roles(3).await;

    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    // Send 10 NEAR to charlie
    let request_id = alice
        .call(&worker, contract.id(), "request")
        .args_json(json!({
            "receiver_id": charlie.id(),
            "actions": [
                PromiseAction::Transfer {
                    amount: (near_sdk::ONE_NEAR * 10).into(),
                },
            ],
        }))
        .unwrap()
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    let is_approved = || async {
        contract
            .view(
                &worker,
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
        .call(&worker, contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .unwrap()
        .transact()
        .await
        .unwrap();

    assert!(!is_approved().await);

    bob.call(&worker, contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .unwrap()
        .transact()
        .await
        .unwrap();

    assert!(is_approved().await);

    charlie
        .call(&worker, contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .unwrap()
        .transact()
        .await
        .unwrap();

    assert!(is_approved().await);

    let balance_before = worker.view_account(charlie.id()).await.unwrap().balance;

    alice
        .call(&worker, contract.id(), "execute")
        .args_json(json!({ "request_id": request_id }))
        .unwrap()
        .transact()
        .await
        .unwrap();

    let balance_after = worker.view_account(charlie.id()).await.unwrap().balance;

    // charlie's balance should have increased by exactly 10 NEAR
    assert_eq!(balance_after - balance_before, near_sdk::ONE_NEAR * 10);
}

#[tokio::test]
async fn reflexive_xcc() {
    let Setup {
        worker,
        contract,
        accounts,
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
        .call(&worker, contract.id(), "request")
        .args_json(json!({
            "receiver_id": contract.id(),
            "actions": actions,
        }))
        .unwrap()
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    alice
        .call(&worker, contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .unwrap()
        .transact()
        .await
        .unwrap();

    bob.call(&worker, contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .unwrap()
        .transact()
        .await
        .unwrap();

    let result = charlie
        .call(&worker, contract.id(), "execute")
        .max_gas()
        .args_json(json!({ "request_id": request_id }))
        .unwrap()
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    assert_eq!(result, 26);
}
