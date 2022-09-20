#![cfg(not(windows))]

use near_contract_tools::approval::native_transaction_action::PromiseAction;
use near_sdk::{serde_json::json, Gas};
use workspaces::{network::Sandbox, prelude::*, Account, Contract, Worker};

const WASM: &[u8] = include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_old.wasm");

const NEW_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_new.wasm");

const BAD_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_bad.wasm");

const RANDOM_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/counter_multisig.wasm");

struct Setup {
    pub worker: Worker<Sandbox>,
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize, wasm: &[u8]) -> Setup {
    let worker = workspaces::sandbox().await.unwrap();

    // Initialize contract
    // let contract = worker.dev_deploy(&wasm.to_vec()).await.unwrap();
    // contract.call(&worker, "new").transact().await.unwrap();

    // Initialize user accounts
    let mut accounts = vec![];
    for _ in 0..(num_accounts + 1) {
        accounts.push(worker.dev_create_account().await.unwrap());
    }

    let alice = &accounts[0].clone();

    let contract = alice
        .deploy(&worker, &wasm.to_vec())
        .await
        .unwrap()
        .unwrap();
    contract.call(&worker, "new").transact().await.unwrap();

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
    } = setup(3, WASM).await;

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
        .call(&worker, contract.id(), "upgrade_all")
        .max_gas()
        .args(NEW_WASM.to_vec())
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

    alice
        .call(&worker, contract.id(), "decrement_bar")
        .args_json(json!({}))
        .unwrap()
        .transact()
        .await
        .unwrap();

    let end_val = alice
        .call(&worker, contract.id(), "get_bar")
        .args_json(json!({}))
        .unwrap()
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    assert_eq!(end_val, 0);
}

#[tokio::test]
#[should_panic = "called `Result::unwrap()` on an `Err` value: Action #1: CompilationError(PrepareError(Deserialization))"]
async fn upgrade_failure_blank_wasm() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup(1, WASM).await;

    let alice = &accounts[0];

    alice
        .call(&worker, contract.id(), "upgrade_all")
        .max_gas()
        .args(vec![])
        .transact()
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic = "called `Result::unwrap()` on an `Err` value: Action #0: MethodResolveError(MethodNotFound)"]
async fn upgrade_failure_no_upgrade() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup(1, BAD_WASM).await;

    let alice = &accounts[0];

    alice
        .call(&worker, contract.id(), "upgrade_all")
        .max_gas()
        .args(vec![])
        .transact()
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic = "called `Result::unwrap()` on an `Err` value: Action #0: MethodResolveError(MethodNotFound)"]
async fn upgrade_failure_random_wasm() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup(1, RANDOM_WASM).await;

    let alice = &accounts[0];

    alice
        .call(&worker, contract.id(), "upgrade_all")
        .max_gas()
        .args(vec![])
        .transact()
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic = "called `Result::unwrap()` on an `Err` value: Action #0: ExecutionError(\"Smart contract panicked: Owner only\")"]
async fn upgrade_failure_not_owner() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup(2, WASM).await;

    let alice = &accounts[0];
    let bob: &Account = &accounts[1];

    bob.call(&worker, contract.id(), "upgrade_all")
        .max_gas()
        .args(vec![])
        .transact()
        .await
        .unwrap();
}
