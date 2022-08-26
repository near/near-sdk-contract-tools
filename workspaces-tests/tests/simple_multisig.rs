#![cfg(not(windows))]

use near_sdk::{
    serde_json::{self, json},
    AccountId,
};
use workspaces::{prelude::*, Account, Contract, Network, Worker};

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/simple_multisig.wasm");

struct Setup<N: Network> {
    pub worker: Worker<N>,
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize) -> Setup<impl Network> {
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
async fn successful_request() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup_roles(3).await;

    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    let request_id = alice
        .call(&worker, contract.id(), "request")
        .args_json(json!({"action": "hello"}))
        .unwrap()
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    let is_approved = || async {
        let r = contract
            .view(
                &worker,
                "is_approved",
                json!({ "request_id": request_id })
                    .to_string()
                    .as_bytes()
                    .to_vec(),
            )
            .await;

        dbg!(&r);
        r.unwrap().json::<bool>().unwrap()
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

    let exec_result = charlie
        .call(&worker, contract.id(), "execute")
        .args_json(json!({ "request_id": request_id }))
        .unwrap()
        .transact()
        .await
        .unwrap()
        .json::<String>()
        .unwrap();

    assert_eq!(exec_result, "hello");
}

#[tokio::test]
#[should_panic = "Unauthorized account"]
async fn unauthorized_account() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup_roles(3).await;

    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];
    let unauthorized_account = &accounts[3];

    let request_id = alice
        .call(&worker, contract.id(), "request")
        .args_json(json!({"action": "hello"}))
        .unwrap()
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    let is_approved = || async {
        let r = contract
            .view(
                &worker,
                "is_approved",
                json!({ "request_id": request_id })
                    .to_string()
                    .as_bytes()
                    .to_vec(),
            )
            .await;

        dbg!(&r);
        r.unwrap().json::<bool>().unwrap()
    };

    assert!(!is_approved().await);

    unauthorized_account
        .call(&worker, contract.id(), "approve")
        .args_json(json!({ "request_id": request_id }))
        .unwrap()
        .transact()
        .await
        .unwrap();
}
