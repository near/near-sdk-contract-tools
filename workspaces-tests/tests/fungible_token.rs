#![cfg(not(windows))]

use near_sdk::{json_types::U128, serde_json::json};
use workspaces::{prelude::*, Account, AccountId, Contract, Network, Worker};

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/fungible_token.wasm");

async fn balance(worker: &Worker<impl Network>, contract: &Contract, account: &AccountId) -> u128 {
    contract
        .view(
            &worker,
            "ft_balance_of",
            json!({ "account_id": account })
                .to_string()
                .as_bytes()
                .to_vec(),
        )
        .await
        .unwrap()
        .json::<U128>()
        .map(|i| u128::from(i))
        .unwrap()
}

struct Setup<N: Network> {
    pub worker: Worker<N>,
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize) -> Setup<impl Network> {
    let worker = workspaces::sandbox().await.unwrap();

    // Initialize contract
    let contract = worker.dev_deploy(&WASM.to_vec()).await.unwrap();
    contract.call(&worker, "new").transact().await.unwrap();

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

async fn setup_balances(
    num_accounts: usize,
    balance: impl Fn(usize) -> U128,
) -> Setup<impl Network> {
    let s = setup(num_accounts).await;

    for (i, account) in s.accounts.iter().enumerate() {
        account
            .call(&s.worker, s.contract.id(), "mint")
            .args_json(json!({ "amount": balance(i) }))
            .unwrap()
            .transact()
            .await
            .unwrap();
    }

    s
}

#[tokio::test]
async fn start_empty() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup(3).await;

    // All accounts must start with 0 balance
    for account in accounts.iter() {
        assert_eq!(balance(&worker, &contract, account.id()).await, 0);
    }
}

#[tokio::test]
async fn mint() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    // Verify issued balances
    assert_eq!(balance(&worker, &contract, alice.id()).await, 1000);
    assert_eq!(balance(&worker, &contract, bob.id()).await, 100);
    assert_eq!(balance(&worker, &contract, charlie.id()).await, 10);
}

#[tokio::test]
async fn transfer_normal() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    alice
        .call(&worker, contract.id(), "ft_transfer")
        .deposit(1)
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "10",
        }))
        .unwrap()
        .transact()
        .await
        .unwrap();
    assert_eq!(balance(&worker, &contract, alice.id()).await, 990);
    assert_eq!(balance(&worker, &contract, bob.id()).await, 110);
    assert_eq!(balance(&worker, &contract, charlie.id()).await, 10);
}

#[tokio::test]
async fn transfer_zero() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    alice
        .call(&worker, contract.id(), "ft_transfer")
        .deposit(1)
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "0",
        }))
        .unwrap()
        .transact()
        .await
        .unwrap();
    assert_eq!(balance(&worker, &contract, alice.id()).await, 1000);
    assert_eq!(balance(&worker, &contract, bob.id()).await, 100);
    assert_eq!(balance(&worker, &contract, charlie.id()).await, 10);
}

#[tokio::test]
#[should_panic(expected = "invalid digit found in string")]
async fn transfer_negative() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    alice
        .call(&worker, contract.id(), "ft_transfer")
        .deposit(1)
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "-10",
        }))
        .unwrap()
        .transact()
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
async fn transfer_no_deposit() {
    let Setup {
        worker,
        contract,
        accounts,
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    alice
        .call(&worker, contract.id(), "ft_transfer")
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "10",
        }))
        .unwrap()
        .transact()
        .await
        .unwrap();
}
