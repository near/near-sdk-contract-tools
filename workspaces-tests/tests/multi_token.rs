use near_sdk::{
    json_types::{Base64VecU8, U128},
    serde_json::json,
    NearToken,
};
use near_sdk_contract_tools::{
    mt::*, standard::nep145::error::InsufficientBalanceError, standard::nep297::Event,
};
use near_workspaces::{network::Sandbox, operations::Function, Account, Contract, Worker};
use pretty_assertions::assert_eq;
use tokio::task::JoinSet;
use workspaces_tests_utils::{
    expect_execution_error, mt_balance_of, mt_batch_balance_of, ONE_NEAR, ONE_YOCTO,
};

const WASM: &[u8] = include_bytes!("../../target/wasm32-unknown-unknown/release/multi_token.wasm");

const RECEIVER_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/multi_token_receiver.wasm");

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
            .call(
                Function::new("mint")
                    .args_json(json!({ "token_id": "my_token", "amount": amount(i) })),
            )
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
        assert_eq!(mt_balance_of(&contract, account.id(), "my_token").await, 0);
    }
}

#[tokio::test]
async fn contract_metadata() {
    let Setup { contract, .. } = setup(3).await;

    let contract_metadata = contract
        .view("mt_metadata_contract")
        .await
        .unwrap()
        .json::<ContractMetadata>()
        .unwrap();
    assert_eq!(
        contract_metadata,
        ContractMetadata {
            spec: ContractMetadata::SPEC.to_string(),
            name: "My MultiToken".to_string()
        },
    );
}

#[tokio::test]
async fn token_metadata() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];

    let base_metadata = BaseMetadata::new("Base Metadata", "base_0");

    alice
        .call(contract.id(), "create_base_meta")
        .args_json(json!({
            "base_metadata": base_metadata,
        }))
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    let token_metadata = TokenMetadata::new()
        .title("Token Title")
        .description("Token Description");

    alice
        .call(contract.id(), "set_token_meta")
        .args_json(json!({
            "token_id": "my_token",
            "base_metadata_id": "base_0",
            "token_metadata": token_metadata,
        }))
        .transact()
        .await
        .unwrap()
        .into_result()
        .unwrap();

    let metadata_token_all = contract
        .view("mt_metadata_token_all")
        .args_json(json!({ "token_ids": ["my_token"] }))
        .await
        .unwrap()
        .json::<Vec<TokenMetadataAll>>()
        .unwrap();
    assert_eq!(
        metadata_token_all,
        vec![TokenMetadataAll {
            base: base_metadata.clone(),
            token: token_metadata.clone()
        }],
    );

    let metadata_token_by_token_id = contract
        .view("mt_metadata_token_by_token_id")
        .args_json(json!({ "token_ids": ["my_token"] }))
        .await
        .unwrap()
        .json::<Vec<TokenMetadata>>()
        .unwrap();
    assert_eq!(metadata_token_by_token_id, vec![token_metadata.clone()]);

    let metadata_base_by_token_id = contract
        .view("mt_metadata_base_by_token_id")
        .args_json(json!({ "token_ids": ["my_token"] }))
        .await
        .unwrap()
        .json::<Vec<BaseMetadata>>()
        .unwrap();
    assert_eq!(metadata_base_by_token_id, vec![base_metadata.clone()]);

    let metadata_base_by_metadata_id = contract
        .view("mt_metadata_base_by_metadata_id")
        .args_json(json!({ "base_metadata_ids": ["base_0"] }))
        .await
        .unwrap()
        .json::<Vec<BaseMetadata>>()
        .unwrap();
    assert_eq!(metadata_base_by_metadata_id, vec![base_metadata.clone()]);
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
    assert_eq!(mt_balance_of(&contract, alice.id(), "my_token").await, 1000);
    assert_eq!(mt_balance_of(&contract, bob.id(), "my_token").await, 100);
    assert_eq!(mt_balance_of(&contract, charlie.id(), "my_token").await, 10);
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
        .call(contract.id(), "mt_transfer")
        .deposit(ONE_YOCTO)
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_id": "my_token",
            "amount": "10",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(mt_balance_of(&contract, alice.id(), "my_token").await, 990);
    assert_eq!(mt_balance_of(&contract, bob.id(), "my_token").await, 110);
    assert_eq!(mt_balance_of(&contract, charlie.id(), "my_token").await, 10);
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Token ID token_dne does not exist."]
async fn transfer_token_id_dne() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(2, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    alice
        .call(contract.id(), "mt_transfer")
        .deposit(ONE_YOCTO)
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_id": "token_dne",
            "amount": "1",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn batch_transfer_normal() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    alice
        .call(contract.id(), "mint")
        .args_json(json!({
            "token_id": "my_token_2",
            "amount": "1000",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    alice
        .call(contract.id(), "mt_batch_transfer")
        .deposit(ONE_YOCTO)
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_ids": ["my_token", "my_token_2"],
            "amounts": ["3", "33"],
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        mt_batch_balance_of(&contract, alice.id(), ["my_token", "my_token_2"]).await,
        [997, 967],
    );
    assert_eq!(
        mt_batch_balance_of(&contract, bob.id(), ["my_token", "my_token_2"]).await,
        [103, 33],
    );
    assert_eq!(
        mt_batch_balance_of(&contract, charlie.id(), ["my_token", "my_token_2"]).await,
        [10, 0],
    );
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
        .call(contract.id(), "mt_transfer")
        .deposit(ONE_YOCTO)
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_id": "my_token",
            "amount": "0",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();
    assert_eq!(mt_balance_of(&contract, alice.id(), "my_token").await, 1000);
    assert_eq!(mt_balance_of(&contract, bob.id(), "my_token").await, 100);
    assert_eq!(mt_balance_of(&contract, charlie.id(), "my_token").await, 10);
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
        .call(contract.id(), "mt_transfer")
        .deposit(ONE_YOCTO)
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_id": "my_token",
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
        .call(contract.id(), "mt_transfer")
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_id": "my_token",
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
        .call(contract.id(), "mt_transfer")
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_id": "my_token",
            "amount": "1000000",
        }))
        .deposit(ONE_YOCTO)
        .transact()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "would overflow u128")]
async fn transfer_overflow_u128() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(2, |_| (u128::MAX / 2).into()).await;
    let alice = &accounts[0];

    alice
        .call(contract.id(), "mint")
        .args_json(json!({
            "token_id": "my_token",
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
        .call(contract.id(), "mt_transfer")
        .deposit(ONE_YOCTO)
        .args_json(json!({
            "receiver_id": charlie.id(),
            "token_id": "my_token",
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
                account_id: alice.id().clone(),
                available: balance.available,
                attempted_to_use: NearToken::from_yoctonear(100490000000000000000000),
            },
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

    bob.deploy(RECEIVER_WASM).await.unwrap().unwrap();

    let result = alice
        .call(contract.id(), "mt_transfer_call")
        .deposit(ONE_YOCTO)
        .max_gas()
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_id": "my_token",
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
            Nep245Event::MtTransfer(vec![MtTransferData {
                authorized_id: None,
                old_owner_id: alice.id().into(),
                new_owner_id: bob.id().into(),
                memo: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.id(),
                alice.id(),
            ),
        ]
    );

    assert_eq!(mt_balance_of(&contract, alice.id(), "my_token").await, 990);
    assert_eq!(mt_balance_of(&contract, bob.id(), "my_token").await, 110);
    assert_eq!(mt_balance_of(&contract, charlie.id(), "my_token").await, 10);
}

#[tokio::test]
async fn batch_transfer_call_normal() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    bob.deploy(RECEIVER_WASM).await.unwrap().unwrap();

    alice
        .call(contract.id(), "mint")
        .args_json(json!({
            "token_id": "my_token_2",
            "amount": "1000",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = alice
        .call(contract.id(), "mt_batch_transfer_call")
        .deposit(ONE_YOCTO)
        .max_gas()
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_ids": ["my_token","my_token_2"],
            "amounts": ["10","20"],
            "msg": "", // keep all of the tokens
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep245Event::MtTransfer(vec![MtTransferData {
                authorized_id: None,
                old_owner_id: alice.id().into(),
                new_owner_id: bob.id().into(),
                memo: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![10.into(), 20.into()],
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.id(),
                alice.id(),
            ),
            format!(
                "Received 20 of my_token_2 from {} via {}",
                alice.id(),
                alice.id(),
            ),
        ]
    );

    assert_eq!(
        mt_batch_balance_of(&contract, alice.id(), ["my_token", "my_token_2"]).await,
        [990, 980],
    );
    assert_eq!(
        mt_batch_balance_of(&contract, bob.id(), ["my_token", "my_token_2"]).await,
        [110, 20],
    );
    assert_eq!(
        mt_batch_balance_of(&contract, charlie.id(), ["my_token", "my_token_2"]).await,
        [10, 0],
    );
}

#[tokio::test]
async fn transfer_call_return() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    bob.deploy(RECEIVER_WASM).await.unwrap().unwrap();

    let result = alice
        .call(contract.id(), "mt_transfer_call")
        .deposit(ONE_YOCTO)
        .max_gas()
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_id": "my_token",
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
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: alice.id().into(),
                new_owner_id: bob.id().into(),
                memo: None,
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.id(),
                alice.id(),
            ),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.id().into(),
                new_owner_id: alice.id().into(),
                memo: None,
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(mt_balance_of(&contract, alice.id(), "my_token").await, 1000);
    assert_eq!(mt_balance_of(&contract, bob.id(), "my_token").await, 100);
    assert_eq!(mt_balance_of(&contract, charlie.id(), "my_token").await, 10);
}

#[tokio::test]
async fn batch_transfer_call_return() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    bob.deploy(RECEIVER_WASM).await.unwrap().unwrap();

    alice
        .call(contract.id(), "mint")
        .args_json(json!({
            "token_id": "my_token_2",
            "amount": "1000",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = alice
        .call(contract.id(), "mt_batch_transfer_call")
        .deposit(ONE_YOCTO)
        .max_gas()
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_ids": ["my_token", "my_token_2"],
            "amounts": ["10", "20"],
            "msg": "return:[\"1\",\"2\"]",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: alice.id().into(),
                new_owner_id: bob.id().into(),
                memo: None,
                authorized_id: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![10.into(), 20.into()],
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.id(),
                alice.id(),
            ),
            format!(
                "Received 20 of my_token_2 from {} via {}",
                alice.id(),
                alice.id(),
            ),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.id().into(),
                new_owner_id: alice.id().into(),
                memo: None,
                authorized_id: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![1.into(), 2.into()],
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(
        mt_batch_balance_of(&contract, alice.id(), ["my_token", "my_token_2"]).await,
        [991, 982],
    );
    assert_eq!(
        mt_batch_balance_of(&contract, bob.id(), ["my_token", "my_token_2"]).await,
        [109, 18],
    );
    assert_eq!(
        mt_batch_balance_of(&contract, charlie.id(), ["my_token", "my_token_2"]).await,
        [10, 0],
    );
}

#[tokio::test]
async fn transfer_call_inner_transfer() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    bob.deploy(RECEIVER_WASM).await.unwrap().unwrap();

    let result = alice
        .call(contract.id(), "mt_transfer_call")
        .deposit(ONE_YOCTO)
        .max_gas()
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_id": "my_token",
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
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: alice.id().into(),
                new_owner_id: bob.id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
                memo: None,
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.id(),
                alice.id(),
            ),
            format!("Transferring all to {}", charlie.id()),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.id().into(),
                new_owner_id: charlie.id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
                memo: None,
            }])
            .to_event_string(),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.id().into(),
                new_owner_id: alice.id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(mt_balance_of(&contract, alice.id(), "my_token").await, 1000);
    assert_eq!(mt_balance_of(&contract, bob.id(), "my_token").await, 90);
    assert_eq!(mt_balance_of(&contract, charlie.id(), "my_token").await, 20);
}

#[tokio::test]
async fn batch_transfer_call_inner_transfer() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    bob.deploy(RECEIVER_WASM).await.unwrap().unwrap();

    alice
        .call(contract.id(), "mint")
        .args_json(json!({
            "token_id": "my_token_2",
            "amount": "1000",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = alice
        .call(contract.id(), "mt_batch_transfer_call")
        .deposit(ONE_YOCTO)
        .max_gas()
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_ids": ["my_token","my_token_2"],
            "amounts": ["10","20"],
            "msg": format!("transfer:{}", charlie.id()),
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: alice.id().into(),
                new_owner_id: bob.id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![10.into(), 20.into()],
                memo: None,
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.id(),
                alice.id(),
            ),
            format!(
                "Received 20 of my_token_2 from {} via {}",
                alice.id(),
                alice.id(),
            ),
            format!("Transferring all to {}", charlie.id()),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.id().into(),
                new_owner_id: charlie.id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![10.into(), 20.into()],
                memo: None,
            }])
            .to_event_string(),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.id().into(),
                new_owner_id: alice.id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(
        mt_batch_balance_of(&contract, alice.id(), ["my_token", "my_token_2"]).await,
        [1000, 1000 - 20],
    );
    assert_eq!(
        mt_batch_balance_of(&contract, bob.id(), ["my_token", "my_token_2"]).await,
        [100 - 10, 0],
    );
    assert_eq!(
        mt_batch_balance_of(&contract, charlie.id(), ["my_token", "my_token_2"]).await,
        [20, 20],
    );
}

#[tokio::test]
async fn transfer_call_inner_panic() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    bob.deploy(RECEIVER_WASM).await.unwrap().unwrap();

    let result = alice
        .call(contract.id(), "mt_transfer_call")
        .deposit(ONE_YOCTO)
        .max_gas()
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_id": "my_token",
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
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: alice.id().into(),
                new_owner_id: bob.id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
                memo: None,
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.id(),
                alice.id(),
            ),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.id().into(),
                new_owner_id: alice.id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(mt_balance_of(&contract, alice.id(), "my_token").await, 1000);
    assert_eq!(mt_balance_of(&contract, bob.id(), "my_token").await, 100);
    assert_eq!(mt_balance_of(&contract, charlie.id(), "my_token").await, 10);
}

#[tokio::test]
async fn batch_transfer_call_inner_panic() {
    let Setup {
        contract, accounts, ..
    } = setup_balances(3, |i| 10u128.pow(3 - i as u32).into()).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    bob.deploy(RECEIVER_WASM).await.unwrap().unwrap();

    alice
        .call(contract.id(), "mint")
        .args_json(json!({
            "token_id": "my_token_2",
            "amount": "1000",
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = alice
        .call(contract.id(), "mt_batch_transfer_call")
        .deposit(ONE_YOCTO)
        .max_gas()
        .args_json(json!({
            "receiver_id": bob.id(),
            "token_ids": ["my_token","my_token_2"],
            "amounts": ["10","20"],
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
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: alice.id().into(),
                new_owner_id: bob.id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![10.into(), 20.into()],
                memo: None,
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.id(),
                alice.id(),
            ),
            format!(
                "Received 20 of my_token_2 from {} via {}",
                alice.id(),
                alice.id(),
            ),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.id().into(),
                new_owner_id: alice.id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![10.into(), 20.into()],
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(
        mt_batch_balance_of(&contract, alice.id(), ["my_token", "my_token_2"]).await,
        [1000, 1000],
    );
    assert_eq!(
        mt_batch_balance_of(&contract, bob.id(), ["my_token", "my_token_2"]).await,
        [100, 0],
    );
    assert_eq!(
        mt_batch_balance_of(&contract, charlie.id(), ["my_token", "my_token_2"]).await,
        [10, 0],
    );
}
