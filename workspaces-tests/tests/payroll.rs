#![cfg(not(windows))]
use near_sdk::serde_json::json;
use std::str;
use workspaces::{result::ExecutionFailure, Account, Contract};

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
    owner
        .call(contract_id, "add_manager")
        .args_json(json!({
            "account_id": manager.id()
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    manager
        .call(contract_id, "add_employee")
        .args_json(json!({
            "account_id": emp1.id(),
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    manager
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

/// Test that setup and employee apis for checking status and logged hours
#[tokio::test]
async fn test_setup() {
    let Setup { contract, accounts } = setup().await;

    let contract_id = contract.id();
    let manager = &accounts[1];
    let emp1 = &accounts[2];

    let result = emp1
        .call(contract_id, "is_employee")
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        str::from_utf8(&result.raw_bytes().unwrap()).unwrap(),
        "true"
    );

    let result = emp1
        .call(contract_id, "get_logged_time")
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(str::from_utf8(&result.raw_bytes().unwrap()).unwrap(), "0");

    // Only employee can log time
    assert!(manager
        .call(contract_id, "get_logged_time")
        .transact()
        .await
        .unwrap()
        .is_failure())
}

/// Test log time correctly logs time for the callee
#[tokio::test]
async fn test_log_time() {
    let Setup { contract, accounts } = setup().await;

    let contract_id = contract.id();
    let emp1 = &accounts[2];
    let emp2 = &accounts[3];

    emp1.call(contract_id, "log_time")
        .args_json(json!({
            "hours": 10,
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    emp2.call(contract_id, "log_time")
        .args_json(json!({
            "hours": 9,
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    emp1.call(contract_id, "log_time")
        .args_json(json!({
            "hours": 10,
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = emp1
        .call(contract_id, "get_logged_time")
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(str::from_utf8(&result.raw_bytes().unwrap()).unwrap(), "20");

    let result = emp2
        .call(contract_id, "get_logged_time")
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(str::from_utf8(&result.raw_bytes().unwrap()).unwrap(), "9");
}

#[tokio::test]
async fn test_disburse_payment() {
    let Setup { contract, accounts } = setup().await;

    let contract_id = contract.id();
    let manager = &accounts[1];
    let emp1 = &accounts[2];

    emp1.call(contract_id, "log_time")
        .args_json(json!({
            "hours": 10,
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let request_id = emp1
        .call(contract_id, "request_pay")
        .transact()
        .await
        .unwrap()
        .unwrap();

    let request_id = str::from_utf8(&request_id.raw_bytes().unwrap())
        .unwrap()
        .parse::<u32>()
        .expect("request_pay response should be an integer");

    manager
        .call(contract_id, "approve_pay")
        .args_json(json!({
            "request_id": request_id,
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let promise = emp1
        .call(contract_id, "log_time")
        .args_json(json!({
            "hours": 10,
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();
}
