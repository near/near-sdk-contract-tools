#![cfg(not(windows))]

use near_sdk::{json_types::U128, serde_json::json};
use near_sdk_contract_tools::standard::nep171::Token;
use workspaces::{Account, AccountId, Contract};

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/non_fungible_token.wasm");

async fn nft_token(contract: &Contract, token_id: &str) -> Option<Token> {
    contract
        .view("nft_token")
        .args_json(json!({ "token_id": token_id }))
        .await
        .unwrap()
        .json::<Option<Token>>()
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

async fn setup_balances(num_accounts: usize, token_ids: impl Fn(usize) -> Vec<String>) -> Setup {
    let s = setup(num_accounts).await;

    for (i, account) in s.accounts.iter().enumerate() {
        account
            .call(s.contract.id(), "mint")
            .args_json(json!({ "token_ids": token_ids(i) }))
            .transact()
            .await
            .unwrap()
            .unwrap();
    }

    s
}

#[tokio::test]
async fn mint() {
    let Setup { contract, accounts } = setup_balances(3, |i| vec![format!("token_{i}")]).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    // Verify minted tokens
    assert_eq!(
        nft_token(&contract, "token_0").await,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: alice.id().parse().unwrap(),
        }),
    );
    assert_eq!(
        nft_token(&contract, "token_1").await,
        Some(Token {
            token_id: "token_1".to_string(),
            owner_id: bob.id().parse().unwrap(),
        }),
    );
    assert_eq!(
        nft_token(&contract, "token_2").await,
        Some(Token {
            token_id: "token_2".to_string(),
            owner_id: charlie.id().parse().unwrap(),
        }),
    );
}
