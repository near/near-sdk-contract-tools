#![cfg(not(windows))]

use near_sdk::serde_json::json;
use near_sdk_contract_tools::standard::{
    nep171::{event::NftTransferLog, Nep171Event, Token},
    nep177::{self, TokenMetadata},
    nep297::Event,
};
use workspaces::operations::Function;
use workspaces_tests_utils::{expect_execution_error, nft_token, setup, Setup};

const WASM_171_ONLY: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/non_fungible_token_nep171.wasm");

const WASM_FULL: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/non_fungible_token_full.wasm");

const RECEIVER_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/non_fungible_token_receiver.wasm");

fn token_meta(id: String) -> near_sdk::serde_json::Value {
    near_sdk::serde_json::to_value(TokenMetadata {
        title: Some(id),
        description: Some("description".to_string()),
        media: None,
        media_hash: None,
        copies: None,
        issued_at: None,
        expires_at: None,
        starts_at: None,
        updated_at: None,
        extra: None,
        reference: None,
        reference_hash: None,
    })
    .unwrap()
}

async fn setup_balances(
    wasm: &[u8],
    num_accounts: usize,
    token_ids: impl Fn(usize) -> Vec<String>,
) -> Setup {
    let s = setup(wasm, num_accounts).await;

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
async fn create_and_mint() {
    let Setup { contract, accounts } =
        setup_balances(WASM_171_ONLY, 3, |i| vec![format!("token_{i}")]).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    let (token_0, token_1, token_2, token_3) = tokio::join!(
        nft_token(&contract, "token_0"),
        nft_token(&contract, "token_1"),
        nft_token(&contract, "token_2"),
        nft_token(&contract, "token_3"),
    );

    // Verify minted tokens
    assert_eq!(
        token_0,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: alice.id().parse().unwrap(),
            extensions_metadata: Default::default(),
        }),
    );
    assert_eq!(
        token_1,
        Some(Token {
            token_id: "token_1".to_string(),
            owner_id: bob.id().parse().unwrap(),
            extensions_metadata: Default::default(),
        }),
    );
    assert_eq!(
        token_2,
        Some(Token {
            token_id: "token_2".to_string(),
            owner_id: charlie.id().parse().unwrap(),
            extensions_metadata: Default::default(),
        }),
    );
    assert_eq!(token_3, None::<Token>);
}

#[tokio::test]
async fn create_and_mint_with_metadata() {
    let Setup { contract, accounts } =
        setup_balances(WASM_FULL, 3, |i| vec![format!("token_{i}")]).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    let metadata = contract
        .view("nft_metadata")
        .await
        .unwrap()
        .json::<Option<nep177::ContractMetadata>>()
        .unwrap()
        .unwrap();

    assert_eq!(
        metadata,
        nep177::ContractMetadata {
            spec: nep177::ContractMetadata::SPEC.to_string(),
            name: "My NFT Smart Contract".to_string(),
            symbol: "MNSC".to_string(),
            icon: None,
            base_uri: None,
            reference: None,
            reference_hash: None,
        },
    );

    let (token_0, token_1, token_2, token_3) = tokio::join!(
        nft_token(&contract, "token_0"),
        nft_token(&contract, "token_1"),
        nft_token(&contract, "token_2"),
        nft_token(&contract, "token_3"),
    );

    // Verify minted tokens
    assert_eq!(
        token_0,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: alice.id().parse().unwrap(),
            extensions_metadata: [
                ("metadata".to_string(), token_meta("token_0".to_string())),
                ("approved_account_ids".to_string(), json!({}),)
            ]
            .into(),
        }),
    );
    assert_eq!(
        token_1,
        Some(Token {
            token_id: "token_1".to_string(),
            owner_id: bob.id().parse().unwrap(),
            extensions_metadata: [
                ("metadata".to_string(), token_meta("token_1".to_string())),
                ("approved_account_ids".to_string(), json!({}),)
            ]
            .into(),
        }),
    );
    assert_eq!(
        token_2,
        Some(Token {
            token_id: "token_2".to_string(),
            owner_id: charlie.id().parse().unwrap(),
            extensions_metadata: [
                ("metadata".to_string(), token_meta("token_2".to_string())),
                ("approved_account_ids".to_string(), json!({}),)
            ]
            .into(),
        }),
    );
    assert_eq!(token_3, None::<Token>);
}

#[tokio::test]
async fn transfer_success() {
    let Setup { contract, accounts } =
        setup_balances(WASM_171_ONLY, 3, |i| vec![format!("token_{i}")]).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    let result = alice
        .call(contract.id(), "nft_transfer")
        .args_json(json!({
            "token_id": "token_0",
            "receiver_id": bob.id(),
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        result.logs(),
        vec![
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                old_owner_id: alice.id().parse().unwrap(),
                new_owner_id: bob.id().parse().unwrap(),
                authorized_id: None,
                memo: None,
                token_ids: vec!["token_0".to_string()],
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
        ],
    );

    let (token_0, token_1, token_2) = tokio::join!(
        nft_token(&contract, "token_0"),
        nft_token(&contract, "token_1"),
        nft_token(&contract, "token_2"),
    );

    assert_eq!(
        token_0,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: bob.id().parse().unwrap(),
            extensions_metadata: Default::default(),
        }),
    );
    assert_eq!(
        token_1,
        Some(Token {
            token_id: "token_1".to_string(),
            owner_id: bob.id().parse().unwrap(),
            extensions_metadata: Default::default(),
        }),
    );
    assert_eq!(
        token_2,
        Some(Token {
            token_id: "token_2".to_string(),
            owner_id: charlie.id().parse().unwrap(),
            extensions_metadata: Default::default(),
        }),
    );
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Requires attached deposit of exactly 1 yoctoNEAR"]
async fn transfer_fail_no_deposit_full() {
    transfer_fail_no_deposit(WASM_FULL).await;
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Requires attached deposit of exactly 1 yoctoNEAR"]
async fn transfer_fail_no_deposit_171() {
    transfer_fail_no_deposit(WASM_171_ONLY).await;
}

async fn transfer_fail_no_deposit(wasm: &[u8]) {
    let Setup { contract, accounts } =
        setup_balances(wasm, 2, |i| vec![format!("token_{i}")]).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    alice
        .call(contract.id(), "nft_transfer")
        .args_json(json!({
            "token_id": "token_0",
            "receiver_id": bob.id(),
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Token `token_5` does not exist"]
async fn transfer_fail_token_dne_full() {
    transfer_fail_token_dne(WASM_FULL).await;
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Token `token_5` does not exist"]
async fn transfer_fail_token_dne_171() {
    transfer_fail_token_dne(WASM_171_ONLY).await;
}

async fn transfer_fail_token_dne(wasm: &[u8]) {
    let Setup { contract, accounts } =
        setup_balances(wasm, 2, |i| vec![format!("token_{i}")]).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    alice
        .call(contract.id(), "nft_transfer")
        .args_json(json!({
            "token_id": "token_5",
            "receiver_id": bob.id(),
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn transfer_fail_not_owner_full() {
    transfer_fail_not_owner(WASM_FULL).await;
}

#[tokio::test]
async fn transfer_fail_not_owner_171() {
    transfer_fail_not_owner(WASM_171_ONLY).await;
}

async fn transfer_fail_not_owner(wasm: &[u8]) {
    let Setup { contract, accounts } =
        setup_balances(wasm, 3, |i| vec![format!("token_{i}")]).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    let result = alice
        .call(contract.id(), "nft_transfer")
        .args_json(json!({
            "token_id": "token_2", // charlie's token
            "receiver_id": bob.id(),
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap();

    expect_execution_error(
        &result,
        format!(
            "Smart contract panicked: Token `token_2` is owned by `{}` instead of expected `{}`",
            charlie.id(),
            alice.id(),
        ),
    );
}

#[tokio::test]
async fn transfer_fail_reflexive_transfer_full() {
    transfer_fail_reflexive_transfer(WASM_FULL).await;
}

#[tokio::test]
async fn transfer_fail_reflexive_transfer_171() {
    transfer_fail_reflexive_transfer(WASM_171_ONLY).await;
}

async fn transfer_fail_reflexive_transfer(wasm: &[u8]) {
    let Setup { contract, accounts } =
        setup_balances(wasm, 2, |i| vec![format!("token_{i}")]).await;
    let alice = &accounts[0];

    let result = alice
        .call(contract.id(), "nft_transfer")
        .args_json(json!({
            "token_id": "token_0",
            "receiver_id": alice.id(),
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap();

    expect_execution_error(&result, format!("Smart contract panicked: Receiver must be different from current owner `{}` to transfer token `token_0`", alice.id()));
}

#[tokio::test]
async fn transfer_call_success() {
    let Setup { contract, accounts } =
        setup_balances(WASM_171_ONLY, 2, |i| vec![format!("token_{i}")]).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    bob.batch(bob.id())
        .deploy(RECEIVER_WASM)
        .call(Function::new("new"))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = alice
        .call(contract.id(), "nft_transfer_call")
        .args_json(json!({
            "token_id": "token_0",
            "receiver_id": bob.id(),
            "msg": "",
        }))
        .gas(30_000_000_000_000)
        .deposit(1)
        .transact()
        .await
        .unwrap()
        .unwrap();

    let logs = result.logs();

    assert_eq!(
        vec![
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".to_string()],
                authorized_id: None,
                old_owner_id: alice.id().parse().unwrap(),
                new_owner_id: bob.id().parse().unwrap(),
                memo: None,
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
            format!("Received token_0 from {} via {}", alice.id(), alice.id()),
        ],
        logs
    );

    // not returned
    assert_eq!(
        nft_token(&contract, "token_0").await,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: bob.id().parse().unwrap(),
            extensions_metadata: Default::default(),
        }),
    );
}

#[tokio::test]
async fn transfer_call_return_success() {
    let Setup { contract, accounts } =
        setup_balances(WASM_171_ONLY, 2, |i| vec![format!("token_{i}")]).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    bob.batch(bob.id())
        .deploy(RECEIVER_WASM)
        .call(Function::new("new"))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = alice
        .call(contract.id(), "nft_transfer_call")
        .args_json(json!({
            "token_id": "token_0",
            "receiver_id": bob.id(),
            "msg": "return",
        }))
        .gas(30_000_000_000_000)
        .deposit(1)
        .transact()
        .await
        .unwrap()
        .unwrap();

    let logs = result.logs();

    assert_eq!(
        vec![
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".to_string()],
                authorized_id: None,
                old_owner_id: alice.id().parse().unwrap(),
                new_owner_id: bob.id().parse().unwrap(),
                memo: None,
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
            format!("Received token_0 from {} via {}", alice.id(), alice.id()),
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".to_string()],
                authorized_id: None,
                old_owner_id: bob.id().parse().unwrap(),
                new_owner_id: alice.id().parse().unwrap(),
                memo: None,
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
        ],
        logs
    );

    // returned
    assert_eq!(
        nft_token(&contract, "token_0").await,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: alice.id().parse().unwrap(),
            extensions_metadata: Default::default(),
        }),
    );
}

#[tokio::test]
async fn transfer_call_receiver_panic() {
    let Setup { contract, accounts } =
        setup_balances(WASM_171_ONLY, 2, |i| vec![format!("token_{i}")]).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    bob.batch(bob.id())
        .deploy(RECEIVER_WASM)
        .call(Function::new("new"))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = alice
        .call(contract.id(), "nft_transfer_call")
        .args_json(json!({
            "token_id": "token_0",
            "receiver_id": bob.id(),
            "msg": "panic",
        }))
        .gas(30_000_000_000_000)
        .deposit(1)
        .transact()
        .await
        .unwrap()
        .unwrap();

    let logs = result.logs();

    assert_eq!(
        vec![
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".to_string()],
                authorized_id: None,
                old_owner_id: alice.id().parse().unwrap(),
                new_owner_id: bob.id().parse().unwrap(),
                memo: None,
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
            format!("Received token_0 from {} via {}", alice.id(), alice.id()),
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".to_string()],
                authorized_id: None,
                old_owner_id: bob.id().parse().unwrap(),
                new_owner_id: alice.id().parse().unwrap(),
                memo: None,
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
        ],
        logs
    );

    // returned
    assert_eq!(
        nft_token(&contract, "token_0").await,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: alice.id().parse().unwrap(),
            extensions_metadata: Default::default(),
        }),
    );
}

#[tokio::test]
async fn transfer_call_receiver_send_return() {
    let Setup { contract, accounts } =
        setup_balances(WASM_171_ONLY, 3, |i| vec![format!("token_{i}")]).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    bob.batch(bob.id())
        .deploy(RECEIVER_WASM)
        .call(Function::new("new"))
        .transact()
        .await
        .unwrap()
        .unwrap();

    let result = alice
        .call(contract.id(), "nft_transfer_call")
        .args_json(json!({
            "token_id": "token_0",
            "receiver_id": bob.id(),
            "msg": format!("transfer:{}", charlie.id()),
        }))
        .gas(300_000_000_000_000) // xtra gas
        .deposit(1)
        .transact()
        .await
        .unwrap()
        .unwrap();

    let logs = result.logs();

    println!("{logs:#?}");

    assert_eq!(
        vec![
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".to_string()],
                authorized_id: None,
                old_owner_id: alice.id().parse().unwrap(),
                new_owner_id: bob.id().parse().unwrap(),
                memo: None,
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
            format!("Received token_0 from {} via {}", alice.id(), alice.id()),
            format!("Transferring token_0 to {}", charlie.id()),
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".to_string()],
                authorized_id: None,
                old_owner_id: bob.id().parse().unwrap(),
                new_owner_id: charlie.id().parse().unwrap(),
                memo: None,
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
            "returning true".to_string(),
        ],
        logs
    );

    // not returned
    assert_eq!(
        nft_token(&contract, "token_0").await,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: charlie.id().parse().unwrap(),
            extensions_metadata: Default::default(),
        }),
    );
}

#[tokio::test]
async fn transfer_approval_success() {
    let Setup { contract, accounts } =
        setup_balances(WASM_FULL, 3, |i| vec![format!("token_{i}")]).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    alice
        .call(contract.id(), "nft_approve")
        .args_json(json!({
            "token_id": "token_0",
            "account_id": bob.id(),
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap()
        .unwrap();

    let view_token = nft_token::<Token>(&contract, "token_0").await;

    let expected_view_token = Token {
        token_id: "token_0".into(),
        owner_id: alice.id().parse().unwrap(),
        extensions_metadata: [
            ("metadata".to_string(), token_meta("token_0".to_string())),
            (
                "approved_account_ids".to_string(),
                json!({
                    bob.id().to_string(): 0,
                }),
            ),
        ]
        .into(),
    };

    assert_eq!(view_token, Some(expected_view_token));

    let is_approved = contract
        .view("nft_is_approved")
        .args_json(json!({
            "token_id": "token_0",
            "approved_account_id": bob.id().to_string(),
        }))
        .await
        .unwrap()
        .json::<bool>()
        .unwrap();

    assert!(is_approved);

    bob.call(contract.id(), "nft_transfer")
        .args_json(json!({
            "token_id": "token_0",
            "approval_id": 0,
            "receiver_id": charlie.id().to_string(),
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        nft_token(&contract, "token_0").await,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: charlie.id().parse().unwrap(),
            extensions_metadata: [
                ("metadata".to_string(), token_meta("token_0".to_string())),
                ("approved_account_ids".to_string(), json!({}))
            ]
            .into(),
        }),
    );
}
