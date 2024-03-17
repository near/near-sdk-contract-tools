#![allow(missing_docs)]

#[cfg(feature = "near-sdk-4")]
extern crate near_sdk_4 as near_sdk;

#[cfg(feature = "near-sdk-5")]
extern crate near_sdk_5 as near_sdk;

use near_sdk::{json_types::U128, serde::de::DeserializeOwned, serde_json::json};
use near_workspaces::{
    result::ExecutionFinalResult, types::NearToken, Account, AccountId, Contract,
};
use pretty_assertions::assert_eq;

pub const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);
pub const ONE_NEAR: NearToken = NearToken::from_near(1);

pub async fn nft_token<T: DeserializeOwned>(contract: &Contract, token_id: &str) -> Option<T> {
    contract
        .view("nft_token")
        .args_json(json!({ "token_id": token_id }))
        .await
        .unwrap()
        .json::<Option<T>>()
        .unwrap()
}

pub async fn ft_balance_of(contract: &Contract, account: &AccountId) -> u128 {
    contract
        .view("ft_balance_of")
        .args_json(json!({ "account_id": account }))
        .await
        .unwrap()
        .json::<U128>()
        .map(u128::from)
        .unwrap()
}

pub struct Setup {
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
pub async fn setup(wasm: &[u8], num_accounts: usize) -> Setup {
    let worker = near_workspaces::sandbox().await.unwrap();

    // Initialize contract
    let contract = worker.dev_deploy(wasm).await.unwrap();
    contract.call("new").transact().await.unwrap().unwrap();

    // Initialize user accounts
    let mut accounts = vec![];
    for _ in 0..num_accounts {
        accounts.push(worker.dev_create_account().await.unwrap());
    }

    Setup { contract, accounts }
}

/// For dynamic should_panic messages
pub fn expect_execution_error(result: &ExecutionFinalResult, expected_error: impl AsRef<str>) {
    let failures = result.failures();

    assert_eq!(failures.len(), 1);

    let actual_error_string = failures[0]
        .clone()
        .into_result()
        .unwrap_err()
        .into_inner()
        .unwrap()
        .to_string();

    assert_eq!(
        format!("Action #0: ExecutionError(\"{}\")", expected_error.as_ref()),
        actual_error_string
    );
}
