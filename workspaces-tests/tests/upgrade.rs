#![cfg(not(windows))]

use near_sdk::borsh::{self, BorshSerialize};
use workspaces::{Account, Contract};

const WASM: &[u8] = include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_old.wasm");

const NEW_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_new.wasm");

const BAD_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_bad.wasm");

const RANDOM_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/counter_multisig.wasm");

#[derive(BorshSerialize)]
struct Args {
    pub code: Vec<u8>,
}

struct Setup {
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize, wasm: &[u8]) -> Setup {
    let worker = workspaces::sandbox().await.unwrap();

    // Initialize user accounts
    let mut accounts = vec![];
    for _ in 0..(num_accounts + 1) {
        accounts.push(worker.dev_create_account().await.unwrap());
    }

    let alice = &accounts[0].clone();

    let contract = alice.deploy(&wasm.to_vec()).await.unwrap().unwrap();
    contract.call("new").transact().await.unwrap().unwrap();

    Setup { contract, accounts }
}

#[tokio::test]
async fn upgrade() {
    let Setup { contract, accounts } = setup(1, WASM).await;

    let alice = &accounts[0];

    alice
        .call(contract.id(), "increment_foo")
        .transact()
        .await
        .unwrap()
        .unwrap();

    let val = alice
        .call(contract.id(), "get_foo")
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    assert_eq!(val, 1);

    alice
        .call(contract.id(), "upgrade")
        .max_gas()
        .args_borsh(Args {
            code: NEW_WASM.to_vec(),
        })
        .transact()
        .await
        .unwrap()
        .unwrap();

    let new_val = alice
        .call(contract.id(), "get_bar")
        .transact()
        .await
        .unwrap()
        .json::<u64>()
        .unwrap();

    assert_eq!(new_val, 1);

    alice
        .call(contract.id(), "decrement_bar")
        .transact()
        .await
        .unwrap()
        .unwrap();

    let end_val = alice
        .call(contract.id(), "get_bar")
        .transact()
        .await
        .unwrap()
        .json::<u32>()
        .unwrap();

    assert_eq!(end_val, 0);
}

#[tokio::test]
#[should_panic = "Failed to deserialize input from Borsh."]
async fn upgrade_failure_blank_wasm() {
    let Setup { contract, accounts } = setup(1, WASM).await;

    let alice = &accounts[0];

    alice
        .call(contract.id(), "upgrade")
        .max_gas()
        .args_borsh([0u8; 0])
        .transact()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
#[should_panic = "MethodResolveError(MethodNotFound)"]
async fn upgrade_failure_no_upgrade() {
    let Setup { contract, accounts } = setup(1, BAD_WASM).await;

    let alice = &accounts[0];

    alice
        .call(contract.id(), "upgrade")
        .max_gas()
        .transact()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
#[should_panic = "MethodResolveError(MethodNotFound)"]
async fn upgrade_failure_random_wasm() {
    let Setup { contract, accounts } = setup(1, RANDOM_WASM).await;

    let alice = &accounts[0];

    alice
        .call(contract.id(), "upgrade")
        .max_gas()
        .transact()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Owner only"]
async fn upgrade_failure_not_owner() {
    let Setup { contract, accounts } = setup(2, WASM).await;

    let bob = &accounts[1];

    bob.call(contract.id(), "upgrade")
        .max_gas()
        .args_borsh(Args {
            code: NEW_WASM.to_vec(),
        })
        .transact()
        .await
        .unwrap()
        .unwrap();
}
