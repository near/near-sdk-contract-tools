#![cfg(not(windows))]

use near_sdk::{
    borsh::{self, BorshSerialize},
    serde::Serialize,
};
use near_workspaces::{Account, Contract};
use pretty_assertions::assert_eq;

const WASM_BORSH: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_old_borsh.wasm");

const WASM_JSON: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_old_jsonbase64.wasm");

const WASM_RAW: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_old_raw.wasm");

const NEW_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_new.wasm");

const BAD_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_bad.wasm");

const RANDOM_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/counter_multisig.wasm");

#[derive(BorshSerialize)]
struct ArgsBorsh {
    pub code: Vec<u8>,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
struct ArgsJson {
    pub code: near_sdk::json_types::Base64VecU8,
}

struct Setup {
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize, wasm: &[u8]) -> Setup {
    let worker = near_workspaces::sandbox().await.unwrap();

    // Initialize user accounts
    let mut accounts = vec![];
    for _ in 0..(num_accounts + 1) {
        accounts.push(worker.dev_create_account().await.unwrap());
    }

    let alice = &accounts[0].clone();

    let contract = alice.deploy(wasm).await.unwrap().unwrap();
    contract.call("new").transact().await.unwrap().unwrap();

    Setup { contract, accounts }
}

async fn perform_upgrade_test(wasm: &[u8], args: Vec<u8>) {
    let Setup { contract, accounts } = setup(1, wasm).await;

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
        .args(args)
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
}

#[tokio::test]
async fn upgrade_borsh() {
    perform_upgrade_test(
        WASM_BORSH,
        ArgsBorsh {
            code: NEW_WASM.to_vec(),
        }
        .try_to_vec()
        .unwrap(),
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn upgrade_jsonbase64() {
    // For some reason this test fails only on GitHub Actions due to a running-out-of-gas error.
    if std::env::var_os("GITHUB_ACTIONS").is_some() {
        eprintln!("Skipping upgrade_jsonbase64 test on GitHub Actions.");
        return;
    }
    perform_upgrade_test(
        WASM_JSON,
        near_sdk::serde_json::to_vec(&ArgsJson {
            code: NEW_WASM.to_vec().into(),
        })
        .unwrap(),
    )
    .await;
}

#[tokio::test]
async fn upgrade_raw() {
    perform_upgrade_test(WASM_RAW, NEW_WASM.to_vec()).await;
}

#[tokio::test]
#[should_panic = "Failed to deserialize input from Borsh."]
async fn upgrade_failure_blank_wasm() {
    perform_upgrade_test(WASM_BORSH, vec![]).await;
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

async fn fail_owner(wasm: &[u8], args: Vec<u8>) {
    let Setup { contract, accounts } = setup(2, wasm).await;

    let bob = &accounts[1];

    bob.call(contract.id(), "upgrade")
        .max_gas()
        .args(args)
        .transact()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Owner only"]
async fn upgrade_failure_not_owner_borsh() {
    fail_owner(
        WASM_BORSH,
        ArgsBorsh {
            code: NEW_WASM.to_vec(),
        }
        .try_to_vec()
        .unwrap(),
    )
    .await;
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Owner only"]
async fn upgrade_failure_not_owner_jsonbase64() {
    fail_owner(
        WASM_JSON,
        near_sdk::serde_json::to_vec(&ArgsJson {
            code: NEW_WASM.to_vec().into(),
        })
        .unwrap(),
    )
    .await;
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Owner only"]
async fn upgrade_failure_not_owner_raw() {
    fail_owner(WASM_RAW, NEW_WASM.to_vec()).await;
}
