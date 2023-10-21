#![cfg(not(windows))]

use near_sdk::{
    json_types::{Base64VecU8, U128},
    serde_json::json,
    ONE_NEAR,
};
use near_workspaces::{network::Sandbox, operations::Function, Account, Contract, Worker};
use tokio::task::JoinSet;
use workspaces_tests_utils::{expect_execution_error, ft_balance_of};

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/fungible_token_nep145.wasm");

struct Setup {
    pub worker: Worker<Sandbox>,
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize) -> Setup {
    let worker = near_workspaces::sandbox().await.unwrap();

    // Initialize contract
    let contract = worker.dev_deploy(WASM).await.unwrap();
    contract.call("new").transact().await.unwrap().unwrap();

    // Initialize user accounts
    let mut accounts = vec![];
    for _ in 0..num_accounts {
        accounts.push(worker.dev_create_account().await.unwrap());
    }

    Setup {
        worker,
        contract,
        accounts,
    }
}

async fn setup_balances(num_accounts: usize, amount: impl Fn(usize) -> U128) -> Setup {
    let setup = setup(num_accounts).await;

    let mut transaction_set = JoinSet::new();

    for (i, account) in setup.accounts.iter().enumerate() {
        let transaction = account
            .batch(setup.contract.id())
            .call(
                Function::new("storage_deposit")
                    .args_json(json!({}))
                    .deposit(ONE_NEAR / 100),
            )
            .call(Function::new("mint").args_json(json!({ "amount": amount(i) })))
            .transact();
        transaction_set.spawn(async move {
            transaction.await.unwrap().unwrap();
        });
    }

    while transaction_set.join_next().await.is_some() {}

    setup
}

#[tokio::test]
async fn transfer_normal() {
    let Setup {
        contract,
        accounts,
        worker: _,
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    alice
        .call(contract.id(), "ft_transfer")
        .deposit(1)
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "10",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ft_balance_of(&contract, alice.id()).await, 990);
    assert_eq!(ft_balance_of(&contract, bob.id()).await, 110);
    assert_eq!(ft_balance_of(&contract, charlie.id()).await, 10);
}

#[tokio::test]
async fn transfer_fail_not_registered() {
    let Setup {
        contract,
        accounts,
        worker,
    } = setup_balances(2, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let charlie = worker.dev_create_account().await.unwrap();

    let result = alice
        .call(contract.id(), "ft_transfer")
        .deposit(1)
        .args_json(json!({
            "receiver_id": charlie.id(),
            "amount": "10",
        }))
        .transact()
        .await
        .unwrap();

    expect_execution_error(
        &result,
        format!(
            "Smart contract panicked: Account {} is not registered",
            charlie.id(),
        ),
    );
}

#[tokio::test]
#[should_panic = "Storage lock error"]
async fn fail_run_out_of_space() {
    let Setup {
        contract,
        accounts,
        worker: _,
    } = setup_balances(2, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];

    for _ in 0..100 {
        alice
            .call(contract.id(), "use_storage")
            .args_json(json!({
                "blob": Base64VecU8::from(vec![1u8; 10000]),
            }))
            .transact()
            .await
            .unwrap()
            .unwrap();
    }
}
