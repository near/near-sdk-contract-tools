#![cfg(not(windows))]

use near_contract_tools::approval::native_transaction_action::PromiseAction;
use near_sdk::{serde_json::json, Gas};
use workspaces::{network::Sandbox, prelude::*, Account, Contract, Worker};

const WASM: &[u8] = include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_old.wasm");

const SECOND_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_new.wasm");

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

async fn setup_roles(num_accounts: usize) -> Setup {
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

/// Setup for individual tests
async fn setup_new(num_accounts: usize) -> Setup {
    let worker = workspaces::sandbox().await.unwrap();

    // Initialize contract
    let contract = worker.dev_deploy(&SECOND_WASM.to_vec()).await.unwrap();
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

#[tokio::test]
async fn upgrade() {
    // Deploy old contract ** LIKE HERE
    // Initialize old contract
    let Setup {
        worker,
        contract,
        accounts,
    } = setup(3).await;

    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    alice
        .call(&worker, contract.id(), "increment_foo")
        .args_json(json!({}))
        .unwrap()
        .transact()
        .await
        .unwrap();

    let val = alice
        .call(&worker, contract.id(), "get_foo")
        .args_json(json!({}))
        .unwrap()
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    assert_eq!(val, 1);


    let request_id = alice
        .call(&worker, contract.id(), "call_upgrade")
        .max_gas()
        .args(SECOND_WASM.to_vec())
        .transact()
        .await
        .unwrap();

    let new_val = alice
        .call(&worker, contract.id(), "get_bar")
        .args_json(json!({}))
        .unwrap()
        .transact()
        .await
        .unwrap()
        .json::<u64>()
        .unwrap();

    assert_eq!(new_val, 1);
}
