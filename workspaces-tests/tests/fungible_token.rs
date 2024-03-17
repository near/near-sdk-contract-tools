workspaces_tests::near_sdk!();

use near_sdk::{
    json_types::{Base64VecU8, U128},
    serde_json::json,
};
use near_sdk_contract_tools::{
    nft::StorageBalance,
    standard::{
        nep141::{FtTransferData, Nep141Event},
        nep145::error::InsufficientBalanceError,
        nep297::Event,
    },
};
use near_workspaces::{network::Sandbox, operations::Function, Account, Contract, Worker};
use pretty_assertions::assert_eq;
use tokio::task::JoinSet;
use workspaces_tests_utils::{expect_execution_error, ft_balance_of, ONE_NEAR, ONE_YOCTO};

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/fungible_token.wasm");

const RECEIVER_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/fungible_token_receiver.wasm");

struct Setup {
    pub contract: Contract,
    pub accounts: Vec<Account>,
    pub worker: Worker<Sandbox>,
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
        contract,
        accounts,
        worker,
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
                    .deposit(ONE_NEAR.saturating_div(100)),
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
    let Setup {
        contract, accounts, ..
    } = setup(3).await;

    // All accounts must start with 0 balance
    for account in accounts.iter() {
        assert_eq!(ft_balance_of(&contract, account.id()).await, 0);
    }
}

#[tokio::test]
async fn mint() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
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
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    alice
        .call(contract.id(), "ft_transfer")
        .deposit(ONE_YOCTO)
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
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    alice
        .call(contract.id(), "ft_transfer")
        .deposit(ONE_YOCTO)
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
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    alice
        .call(contract.id(), "ft_transfer")
        .deposit(ONE_YOCTO)
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
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
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
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    alice
        .call(contract.id(), "ft_transfer")
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "1000000",
        }))
        .deposit(ONE_YOCTO)
        .transact()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "TotalSupplyOverflowError")]
async fn transfer_overflow_u128() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(2, |_| (u128::MAX / 2).into()).await;
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
        .deposit(ONE_YOCTO)
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
async fn fail_run_out_of_space() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(2, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];

    let balance = contract
        .view("storage_balance_of")
        .args_json(json!({ "account_id": alice.id() }))
        .await
        .unwrap()
        .json::<Option<StorageBalance>>()
        .unwrap()
        .unwrap();

    let result = alice
        .call(contract.id(), "use_storage")
        .args_json(json!({
            "blob": Base64VecU8::from(vec![1u8; 10000]),
        }))
        .transact()
        .await
        .unwrap();

    expect_execution_error(
        &result,
        format!(
            "Smart contract panicked: Storage lock error: {}",
            InsufficientBalanceError {
                account_id: alice.id().as_str().parse().unwrap(),
                available: balance.available,
                attempted_to_lock: 100490000000000000000000u128.into()
            }
        ),
    );
}

#[tokio::test]
async fn transfer_call_normal() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    bob.batch(bob.id())
        .deploy(RECEIVER_WASM)
        .call(Function::new("new").args_json(json!({})))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = alice
        .call(contract.id(), "ft_transfer_call")
        .deposit(ONE_YOCTO)
        .max_gas()
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "10",
            "msg": "", // keep all of the tokens
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: alice.id().as_str().parse().unwrap(),
                new_owner_id: bob.id().as_str().parse().unwrap(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
            format!("Received 10 from {}", alice.id()),
        ]
    );

    assert_eq!(ft_balance_of(&contract, alice.id()).await, 990);
    assert_eq!(ft_balance_of(&contract, bob.id()).await, 110);
    assert_eq!(ft_balance_of(&contract, charlie.id()).await, 10);
}

#[tokio::test]
async fn transfer_call_return() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    bob.batch(bob.id())
        .deploy(RECEIVER_WASM)
        .call(Function::new("new").args_json(json!({})))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = alice
        .call(contract.id(), "ft_transfer_call")
        .deposit(ONE_YOCTO)
        .max_gas()
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "10",
            "msg": "return", // return all of the tokens
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: alice.id().as_str().parse().unwrap(),
                new_owner_id: bob.id().as_str().parse().unwrap(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
            format!("Received 10 from {}", alice.id()),
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: bob.id().as_str().parse().unwrap(),
                new_owner_id: alice.id().as_str().parse().unwrap(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(ft_balance_of(&contract, alice.id()).await, 1000);
    assert_eq!(ft_balance_of(&contract, bob.id()).await, 100);
    assert_eq!(ft_balance_of(&contract, charlie.id()).await, 10);
}

#[tokio::test]
async fn transfer_call_inner_transfer() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    bob.batch(bob.id())
        .deploy(RECEIVER_WASM)
        .call(Function::new("new").args_json(json!({})))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = alice
        .call(contract.id(), "ft_transfer_call")
        .deposit(ONE_YOCTO)
        .max_gas()
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "10",
            "msg": format!("transfer:{}", charlie.id()),
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: alice.id().as_str().parse().unwrap(),
                new_owner_id: bob.id().as_str().parse().unwrap(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
            format!("Received 10 from {}", alice.id()),
            format!("Transferring 10 to {}", charlie.id()),
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: bob.id().as_str().parse().unwrap(),
                new_owner_id: charlie.id().as_str().parse().unwrap(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: bob.id().as_str().parse().unwrap(),
                new_owner_id: alice.id().as_str().parse().unwrap(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(ft_balance_of(&contract, alice.id()).await, 1000);
    assert_eq!(ft_balance_of(&contract, bob.id()).await, 90);
    assert_eq!(ft_balance_of(&contract, charlie.id()).await, 20);
}

#[tokio::test]
async fn transfer_call_inner_panic() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    bob.batch(bob.id())
        .deploy(RECEIVER_WASM)
        .call(Function::new("new").args_json(json!({})))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = alice
        .call(contract.id(), "ft_transfer_call")
        .deposit(ONE_YOCTO)
        .max_gas()
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": "10",
            "msg": "panic",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let inner_outcome = result.outcomes().to_vec()[2];

    assert!(inner_outcome.is_failure());

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: alice.id().as_str().parse().unwrap(),
                new_owner_id: bob.id().as_str().parse().unwrap(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
            format!("Received 10 from {}", alice.id()),
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: bob.id().as_str().parse().unwrap(),
                new_owner_id: alice.id().as_str().parse().unwrap(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(ft_balance_of(&contract, alice.id()).await, 1000);
    assert_eq!(ft_balance_of(&contract, bob.id()).await, 100);
    assert_eq!(ft_balance_of(&contract, charlie.id()).await, 10);
}
