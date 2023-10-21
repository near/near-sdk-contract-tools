#![cfg(not(windows))]

use near_sdk::{json_types::U128, serde_json::json, ONE_NEAR};
use near_workspaces::{operations::Function, Account, Contract};
use pretty_assertions::assert_eq;
use tokio::task::JoinSet;
use workspaces_tests_utils::ft_balance_of;

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/fungible_token.wasm");

struct Setup {
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

    Setup { contract, accounts }
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
async fn start_empty() {
    let Setup { contract, accounts } = setup(3).await;

    // All accounts must start with 0 balance
    for account in accounts.iter() {
        assert_eq!(ft_balance_of(&contract, account.id()).await, 0);
    }
}

#[tokio::test]
async fn mint() {
    let Setup { contract, accounts } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    // Verify issued balances
    assert_eq!(ft_balance_of(&contract, alice.id()).await, 1000);
    assert_eq!(ft_balance_of(&contract, bob.id()).await, 100);
    assert_eq!(ft_balance_of(&contract, charlie.id()).await, 10);
}

#[tokio::test]
async fn transfer_normal() {
    let Setup { contract, accounts } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
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
async fn transfer_zero() {
    let Setup { contract, accounts } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    alice
        .call(contract.id(), "ft_transfer")
        .deposit(1)
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "0",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(ft_balance_of(&contract, alice.id()).await, 1000);
    assert_eq!(ft_balance_of(&contract, bob.id()).await, 100);
    assert_eq!(ft_balance_of(&contract, charlie.id()).await, 10);
}

#[tokio::test]
#[should_panic(expected = "invalid digit found in string")]
async fn transfer_negative() {
    let Setup { contract, accounts } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    alice
        .call(contract.id(), "ft_transfer")
        .deposit(1)
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "-10",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
async fn transfer_no_deposit() {
    let Setup { contract, accounts } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    alice
        .call(contract.id(), "ft_transfer")
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "10",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "Balance of the sender is insufficient")]
async fn transfer_more_than_balance() {
    let Setup { contract, accounts } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    alice
        .call(contract.id(), "ft_transfer")
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "1000000",
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "TotalSupplyOverflowError")]
async fn transfer_overflow_u128() {
    let Setup { contract, accounts } = setup_balances(2, |_| (u128::MAX / 2).into()).await;
    let alice = &accounts[0];

    alice
        .call(contract.id(), "mint")
        .args_json(json!({
            "amount": "2",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();
}
