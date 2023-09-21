#![cfg(not(windows))]

use near_sdk::{json_types::U128, serde_json::json};
use workspaces::{Account, AccountId, Contract};

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/fungible_token_nep145.wasm");

async fn balance(contract: &Contract, account: &AccountId) -> u128 {
    contract
        .view("ft_balance_of")
        .args_json(json!({ "account_id": account }))
        .await
        .unwrap()
        .json::<U128>()
        .map(u128::from)
        .unwrap()
}

struct Setup {
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize) -> Setup {
    let worker = workspaces::sandbox().await.unwrap();

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

async fn setup_balances(num_accounts: usize, balance: impl Fn(usize) -> U128) -> Setup {
    let s = setup(num_accounts).await;

    for (i, account) in s.accounts.iter().enumerate() {
        account
            .call(s.contract.id(), "mint")
            .args_json(json!({ "amount": balance(i) }))
            .transact()
            .await
            .unwrap()
            .unwrap();
    }

    s
}

#[tokio::test]
async fn start_empty() {
    let Setup { contract, accounts } = setup(3).await;

    // All accounts must start with 0 balance
    for account in accounts.iter() {
        assert_eq!(balance(&contract, account.id()).await, 0);
    }
}

#[tokio::test]
async fn mint() {
    let Setup { contract, accounts } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    // Verify issued balances
    assert_eq!(balance(&contract, alice.id()).await, 1000);
    assert_eq!(balance(&contract, bob.id()).await, 100);
    assert_eq!(balance(&contract, charlie.id()).await, 10);
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
    assert_eq!(balance(&contract, alice.id()).await, 990);
    assert_eq!(balance(&contract, bob.id()).await, 110);
    assert_eq!(balance(&contract, charlie.id()).await, 10);
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
    assert_eq!(balance(&contract, alice.id()).await, 1000);
    assert_eq!(balance(&contract, bob.id()).await, 100);
    assert_eq!(balance(&contract, charlie.id()).await, 10);
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
