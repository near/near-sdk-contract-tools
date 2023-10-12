#![cfg(not(windows))]

use near_sdk::{json_types::Base64VecU8, serde_json::json};
use near_workspaces::{Account, Contract};
use pretty_assertions::assert_eq;

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_old_multisig.wasm");

const NEW_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/upgrade_new.wasm");

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

#[tokio::test]
async fn upgrade_multisig() {
    let Setup { contract, accounts } = setup(1, WASM).await;

    let alice = &accounts[0];

    let code = Base64VecU8::from(Vec::from(NEW_WASM));

    let request_id: u32 = alice
        .call(contract.id(), "request")
        .max_gas()
        .args_json(json!({
            "request": {
                "Upgrade": {
                    "code": code,
                },
            },
        }))
        .transact()
        .await
        .unwrap()
        .unwrap()
        .json()
        .unwrap();

    alice
        .call(contract.id(), "approve")
        .max_gas()
        .args_json(json!({
            "request_id": request_id,
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    alice
        .call(contract.id(), "execute")
        .max_gas()
        .args_json(json!({
            "request_id": request_id,
        }))
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

    assert_eq!(new_val, 0);
}
