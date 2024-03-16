workspaces_tests::near_sdk!();

use std::collections::HashMap;

use near_sdk::{json_types::U128, serde_json::json};
use near_sdk_contract_tools::{
    compat_near_to_u128,
    standard::{
        nep171::{
            self,
            event::{Nep171Event, NftTransferLog},
            Token,
        },
        nep177::{self, TokenMetadata},
        nep178::error::{
            AccountAlreadyApprovedError, Nep178ApproveError, TooManyApprovalsError,
            UnauthorizedError,
        },
        nep297::Event,
    },
    COMPAT_ONE_NEAR,
};
use near_workspaces::{operations::Function, types::Gas};
use pretty_assertions::assert_eq;
use tokio::task::JoinSet;
use workspaces_tests_utils::{expect_execution_error, nft_token, setup, Setup};

const WASM_171_ONLY: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/non_fungible_token_nep171.wasm");

const WASM_FULL: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/non_fungible_token_full.wasm");

const RECEIVER_WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/non_fungible_token_receiver.wasm");

const THIRTY_TERAGAS: Gas = Gas::from_gas(30_000_000_000_000);

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
    storage_deposit: bool,
) -> Setup {
    let s = setup(wasm, num_accounts).await;

    for (i, account) in s.accounts.iter().enumerate() {
        let batch = if storage_deposit {
            account.batch(s.contract.id()).call(
                Function::new("storage_deposit")
                    .args_json(json!({}))
                    .deposit(compat_near_to_u128!(COMPAT_ONE_NEAR.saturating_div(100))),
            )
        } else {
            account.batch(s.contract.id())
        };

        batch
            .call(Function::new("mint").args_json(json!({ "token_ids": token_ids(i) })))
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
        setup_balances(WASM_171_ONLY, 3, |i| vec![format!("token_{i}")], false).await;
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
async fn create_and_mint_with_metadata_and_enumeration() {
    let Setup { contract, accounts } =
        setup_balances(WASM_FULL, 3, |i| vec![format!("token_{i}")], true).await;
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

    // indeterminate order, so hashmap for equality instead of vec
    let (
        all_tokens_enumeration,
        all_tokens_enumeration_limit,
        alice_supply,
        alice_tokens_all,
        alice_tokens_offset,
        nonexistent_account_tokens,
    ) = tokio::join!(
        async {
            contract
                .view("nft_tokens")
                .args_json(json!({}))
                .await
                .unwrap()
                .json::<Vec<Token>>()
                .unwrap()
                .into_iter()
                .map(|token| (token.token_id.clone(), token))
                .collect::<HashMap<_, _>>()
        },
        async {
            contract
                .view("nft_tokens")
                .args_json(json!({ "from_index": "0", "limit": 100 }))
                .await
                .unwrap()
                .json::<Vec<Token>>()
                .unwrap()
                .into_iter()
                .map(|token| (token.token_id.clone(), token))
                .collect::<HashMap<_, _>>()
        },
        async {
            contract
                .view("nft_supply_for_owner")
                .args_json(json!({ "account_id": alice.id() }))
                .await
                .unwrap()
                .json::<U128>()
                .unwrap()
        },
        async {
            contract
                .view("nft_tokens_for_owner")
                .args_json(json!({ "account_id": alice.id(), "limit": 100 }))
                .await
                .unwrap()
                .json::<Vec<Token>>()
                .unwrap()
        },
        async {
            contract
                .view("nft_tokens_for_owner")
                .args_json(json!({ "account_id": alice.id(), "from_index": "1" }))
                .await
                .unwrap()
                .json::<Vec<Token>>()
                .unwrap()
        },
        async {
            contract
                .view("nft_tokens_for_owner")
                .args_json(json!({ "account_id": "0000000000000000000000000000000000000000000000000000000000000000", "from_index": "1" }))
                .await
                .unwrap()
                .json::<Vec<Token>>()
                .unwrap()
        },
    );

    assert_eq!(
        all_tokens_enumeration,
        [
            token_0.clone().unwrap(),
            token_1.clone().unwrap(),
            token_2.clone().unwrap(),
        ]
        .into_iter()
        .map(|token| (token.token_id.clone(), token))
        .collect::<HashMap<_, _>>(),
    );

    assert_eq!(
        all_tokens_enumeration, all_tokens_enumeration_limit,
        "only 3 tokens in circulation, so limit:100 should be the same as unlimited"
    );

    assert_eq!(
        alice_supply.0, 1,
        "alice has one token, so alice's supply (balance) should be 1"
    );

    assert_eq!(
        alice_tokens_all,
        vec![token_0.clone().unwrap()],
        "alice has one token, so it should be returned in the list of all of alice's tokens"
    );

    assert_eq!(
        alice_tokens_offset,
        vec![],
        "alice only has one token so an offset:1 should return empty"
    );

    assert_eq!(
        nonexistent_account_tokens,
        vec![],
        "nonexistent account should return empty",
    );
}

#[tokio::test]
async fn transfer_success() {
    let Setup { contract, accounts } =
        setup_balances(WASM_171_ONLY, 3, |i| vec![format!("token_{i}")], false).await;
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
    transfer_fail_no_deposit(WASM_FULL, true).await;
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Requires attached deposit of exactly 1 yoctoNEAR"]
async fn transfer_fail_no_deposit_171() {
    transfer_fail_no_deposit(WASM_171_ONLY, false).await;
}

async fn transfer_fail_no_deposit(wasm: &[u8], storage_deposit: bool) {
    let Setup { contract, accounts } =
        setup_balances(wasm, 2, |i| vec![format!("token_{i}")], storage_deposit).await;
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
    transfer_fail_token_dne(WASM_FULL, true).await;
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Token `token_5` does not exist"]
async fn transfer_fail_token_dne_171() {
    transfer_fail_token_dne(WASM_171_ONLY, false).await;
}

async fn transfer_fail_token_dne(wasm: &[u8], storage_deposit: bool) {
    let Setup { contract, accounts } =
        setup_balances(wasm, 2, |i| vec![format!("token_{i}")], storage_deposit).await;
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
    transfer_fail_not_owner(WASM_FULL, true).await;
}

#[tokio::test]
async fn transfer_fail_not_owner_171() {
    transfer_fail_not_owner(WASM_171_ONLY, false).await;
}

async fn transfer_fail_not_owner(wasm: &[u8], storage_deposit: bool) {
    let Setup { contract, accounts } =
        setup_balances(wasm, 3, |i| vec![format!("token_{i}")], storage_deposit).await;
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
    transfer_fail_reflexive_transfer(WASM_FULL, true).await;
}

#[tokio::test]
async fn transfer_fail_reflexive_transfer_171() {
    transfer_fail_reflexive_transfer(WASM_171_ONLY, false).await;
}

async fn transfer_fail_reflexive_transfer(wasm: &[u8], storage_deposit: bool) {
    let Setup { contract, accounts } =
        setup_balances(wasm, 2, |i| vec![format!("token_{i}")], storage_deposit).await;
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
        setup_balances(WASM_171_ONLY, 2, |i| vec![format!("token_{i}")], false).await;
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
        .gas(THIRTY_TERAGAS)
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
        setup_balances(WASM_171_ONLY, 2, |i| vec![format!("token_{i}")], false).await;
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
        .gas(THIRTY_TERAGAS)
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
        setup_balances(WASM_171_ONLY, 2, |i| vec![format!("token_{i}")], false).await;
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
        .gas(THIRTY_TERAGAS)
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
        setup_balances(WASM_171_ONLY, 3, |i| vec![format!("token_{i}")], false).await;
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
        .gas(THIRTY_TERAGAS.saturating_mul(10)) // xtra gas
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
        setup_balances(WASM_FULL, 3, |i| vec![format!("token_{i}")], true).await;
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

#[tokio::test]
async fn transfer_approval_unapproved_fail() {
    let Setup { contract, accounts } =
        setup_balances(WASM_FULL, 4, |i| vec![format!("token_{i}")], true).await;
    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];
    let debbie = &accounts[3];

    alice
        .call(contract.id(), "nft_approve")
        .args_json(json!({
            "token_id": "token_0",
            "account_id": debbie.id(),
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap()
        .unwrap();

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

    assert!(!is_approved);

    let result = bob
        .call(contract.id(), "nft_transfer")
        .args_json(json!({
            "token_id": "token_0",
            "approval_id": 0,
            "receiver_id": charlie.id().to_string(),
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap();

    let expected_error_message = format!(
        "Smart contract panicked: {}",
        nep171::error::SenderNotApprovedError {
            owner_id: alice.id().parse().unwrap(),
            sender_id: bob.id().parse().unwrap(),
            token_id: "token_0".to_string(),
            approval_id: 0,
        }
    );

    expect_execution_error(&result, expected_error_message);
}

#[tokio::test]
#[should_panic = "Attached deposit must be greater than zero"]
async fn transfer_approval_no_deposit_fail() {
    let Setup { contract, accounts } =
        setup_balances(WASM_FULL, 2, |i| vec![format!("token_{i}")], true).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    alice
        .call(contract.id(), "nft_approve")
        .args_json(json!({
            "token_id": "token_0",
            "account_id": bob.id(),
        }))
        .transact()
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn transfer_approval_double_approval_fail() {
    let Setup { contract, accounts } =
        setup_balances(WASM_FULL, 2, |i| vec![format!("token_{i}")], true).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

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

    let result = alice
        .call(contract.id(), "nft_approve")
        .args_json(json!({
            "token_id": "token_0",
            "account_id": bob.id(),
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap();

    let expected_error = format!(
        "Smart contract panicked: {}",
        Nep178ApproveError::AccountAlreadyApproved(AccountAlreadyApprovedError {
            account_id: bob.id().parse().unwrap(),
            token_id: "token_0".to_string(),
        }),
    );

    expect_execution_error(&result, expected_error);
}

#[tokio::test]
async fn transfer_approval_unauthorized_approval_fail() {
    let Setup { contract, accounts } =
        setup_balances(WASM_FULL, 2, |i| vec![format!("token_{i}")], true).await;
    let _alice = &accounts[0];
    let bob = &accounts[1];

    let result = bob
        .call(contract.id(), "nft_approve")
        .args_json(json!({
            "token_id": "token_0",
            "account_id": bob.id(),
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap();

    let expected_error = format!(
        "Smart contract panicked: {}",
        Nep178ApproveError::Unauthorized(UnauthorizedError {
            account_id: bob.id().parse().unwrap(),
            token_id: "token_0".to_string(),
        }),
    );

    expect_execution_error(&result, expected_error);
}

#[tokio::test]
async fn transfer_approval_too_many_approvals_fail() {
    let Setup { contract, accounts } =
        setup_balances(WASM_FULL, 2, |i| vec![format!("token_{i}")], true).await;
    let alice = &accounts[0];
    let bob = &accounts[1];

    let mut set = JoinSet::new();

    for i in 0..32 {
        let contract = contract.clone();
        let alice = alice.clone();
        set.spawn(async move {
            alice
                .call(contract.id(), "nft_approve")
                .args_json(json!({
                    "token_id": "token_0",
                    "account_id": format!("account_{}", i),
                }))
                .deposit(1)
                .transact()
                .await
                .unwrap()
                .unwrap();
        });
    }

    while (set.join_next().await).is_some() {}

    let result = alice
        .call(contract.id(), "nft_approve")
        .args_json(json!({
            "token_id": "token_0",
            "account_id": bob.id(),
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap();

    let expected_error = format!(
        "Smart contract panicked: {}",
        Nep178ApproveError::TooManyApprovals(TooManyApprovalsError {
            token_id: "token_0".to_string(),
        }),
    );

    expect_execution_error(&result, expected_error);
}

#[tokio::test]
async fn transfer_approval_approved_but_wrong_approval_id_fail() {
    let Setup { contract, accounts } =
        setup_balances(WASM_FULL, 3, |i| vec![format!("token_{i}")], true).await;
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

    let result = bob
        .call(contract.id(), "nft_transfer")
        .args_json(json!({
            "token_id": "token_0",
            "approval_id": 1,
            "receiver_id": charlie.id().to_string(),
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap();

    let expected_error = format!(
        "Smart contract panicked: {}",
        nep171::error::Nep171TransferError::SenderNotApproved(
            nep171::error::SenderNotApprovedError {
                sender_id: bob.id().parse().unwrap(),
                owner_id: alice.id().parse().unwrap(),
                token_id: "token_0".to_string(),
                approval_id: 1,
            }
        ),
    );

    expect_execution_error(&result, expected_error);
}

#[tokio::test]
#[should_panic = "Account this_account_is_not_registered.near is not registered"]
async fn transfer_fail_not_registered_nep145() {
    let Setup { contract, accounts } =
        setup_balances(WASM_FULL, 1, |i| vec![format!("token_{i}")], true).await;
    let alice = &accounts[0];

    alice
        .call(contract.id(), "nft_transfer")
        .args_json(json!({
            "token_id": "token_0",
            "receiver_id": "this_account_is_not_registered.near",
        }))
        .deposit(1)
        .transact()
        .await
        .unwrap()
        .unwrap();
}
