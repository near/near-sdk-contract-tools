#![cfg(not(windows))]

use near_contract_tools::upgrade::upgrade;
use near_sdk::{env, json_types::Base64VecU8, serde_json::json};
use workspaces::{Account, Contract};

const WASM: &[u8] = include_bytes!("../../target/wasm32-unknown-unknown/release/first.wasm");

const NEW_WASM: &[u8] = include_bytes!("../../target/wasm32-unknown-unknown/release/second.wasm");

struct Setup {
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize, wasm: &[u8]) -> Setup {
    let worker = workspaces::testnet().await.unwrap();

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
async fn upgrade_multisig() {
    let Setup { contract, accounts } = setup(1, WASM).await;

    let alice = &accounts[0];

    let code = Base64VecU8::from(Vec::from(NEW_WASM));

    env::log_str("creating promise");

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

    env::log_str("Approving ...");

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

    env::log_str("Executing ...");

    let res = alice
        .call(contract.id(), "execute")
        .max_gas()
        .args_json(json!({
            "request_id": request_id,
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    env::log_str("Done Executing ...");

    assert_eq!(
        res.logs(),
        vec!["executing request", "creating promise", "migrate called!"]
    );

    let hello: String = alice
        .view(contract.id(), "say_hello", vec![])
        .await
        .unwrap()
        .json()
        .unwrap();

    assert_eq!(hello, "I am the second contract");
}

#[tokio::test]
async fn upgrade_idk_blank_wasm() {
    let Setup { contract, accounts } = setup(1, WASM).await;

    let alice = &accounts[0];

    alice
        .call(contract.id(), "upgrade_contract")
        .max_gas()
        .transact()
        .await
        .unwrap()
        .unwrap();
}
