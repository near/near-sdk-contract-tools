#![cfg(not(windows))]
use near_contract_tools::DefaultStorageKey;
use near_sdk::serde_json::json;
use workspaces::{Account, Contract};

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/payroll_example.wasm");

struct Setup {
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup() -> Setup {
    let worker = workspaces::sandbox().await.unwrap();

    // Initialize user accounts
    let mut accounts = Vec::new();
    for _ in 0..4 {
        accounts.push(worker.dev_create_account().await.unwrap());
    }
    let contract = worker.dev_deploy(&WASM.to_vec()).await.unwrap();

    let contract_id = contract.id();
    let owner = &accounts[0];
    let manager = &accounts[1];
    let emp1 = &accounts[2];
    let emp2 = &accounts[3];

    // Initialize contract
    contract
        .call("new")
        .args_json(json!({
            "owner": owner.id()
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    // setup roles
    let result = owner
        .call(contract_id, "add_manager")
        .args_json(json!({
            "account_id": manager.id()
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = manager
        .call(contract_id, "add_employee")
        .args_json(json!({
            "account_id": emp1.id(),
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = manager
        .call(contract_id, "add_employee")
        .args_json(json!({
            "account_id": emp2.id(),
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    Setup { contract, accounts }
}

#[tokio::test]
async fn successful_request() {
    let Setup { contract, accounts } = setup().await;

    let contract_id = contract.id();
    let owner = &accounts[0];
    let manager = &accounts[1];
    let emp1 = &accounts[2];
    let emp2 = &accounts[3];

    let result = emp1
        .call(contract_id, "is_employee")
        .transact()
        .await
        .unwrap()
        .unwrap();

    dbg!(&result.borsh::<bool>(), true);

    let result = emp1
        .call(contract_id, "get_logged_time")
        .transact()
        .await
        .unwrap()
        .unwrap();

    dbg!(&result.borsh::<u8>(), 0);
}
