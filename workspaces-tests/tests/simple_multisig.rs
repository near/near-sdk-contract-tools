#![cfg(not(windows))]

use near_sdk::serde_json::json;
use workspaces::{Account, Contract};

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/simple_multisig.wasm");

struct Setup {
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize) -> Setup {
    let worker = workspaces::sandbox().await.unwrap();

    // Initialize contract
    let contract = worker.dev_deploy(&WASM.to_vec()).await.unwrap();
    contract.call("new").transact().await.unwrap().unwrap();

    // Initialize user accounts
    let mut accounts = vec![];
    for _ in 0..(num_accounts + 1) {
        accounts.push(worker.dev_create_account().await.unwrap());
    }

    Setup { contract, accounts }
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
async fn successful_request() {
    let Setup { contract, accounts } = setup_roles(3).await;

    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    let request_id = alice
        .call(contract.id(), "request")
        .args_json(json!({"action": "hello"}))
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

    let exec_result = charlie
        .call(contract.id(), "execute")
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .json::<String>()
        .unwrap();

    assert_eq!(exec_result, "hello");
}

#[tokio::test]
#[should_panic = "UnauthorizedAccount"]
async fn unauthorized_account() {
    let Setup { contract, accounts } = setup_roles(3).await;

    let alice = &accounts[0];
    let unauthorized_account = &accounts[3];

    let request_id = alice
        .call(contract.id(), "request")
        .args_json(json!({"action": "hello"}))
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    unauthorized_account
        .call(contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .transact()
        .await
        .unwrap()
        .unwrap();
}
