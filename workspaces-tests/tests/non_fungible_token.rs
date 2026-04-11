use std::collections::HashMap;

use near_api::{Account, Contract};
use near_sdk::{AccountId, json_types::U128, serde_json::json};
use near_sdk_contract_tools::{
    nft::{ApprovalId, ContractMetadata, TokenId},
    standard::{
        nep171::{
            Token,
            event::{Nep171Event, NftTransferLog},
        },
        nep177::{self, TokenMetadata},
        nep297::Event,
    },
};
use pretty_assertions::assert_eq;
use testresult::TestResult;
use workspaces_tests::{Handle, ONE_NEAR, Y, read_only, transaction};

const CONTRACT_171: &str = "non_fungible_token_nep171";
const CONTRACT_FULL: &str = "non_fungible_token_full";
const CONTRACT_RECEIVER: &str = "non_fungible_token_receiver";

fn token_meta(id: impl Into<String>) -> near_sdk::serde_json::Value {
    near_sdk::serde_json::to_value(TokenMetadata::new().title(id).description("description"))
        .unwrap()
}

struct Setup {
    pub handle: Handle,
    pub contract: Contract,
}

async fn setup(contract: &str) -> TestResult<Setup> {
    let handle = Handle::new().await;
    let contract = handle.make_contract(contract, contract, json!({})).await?;
    Ok(Setup { handle, contract })
}

impl Setup {
    async fn setup_account(
        &self,
        account: impl Into<String>,
        storage_deposit: bool,
        token_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> TestResult<Account> {
        let account = self.handle.make_account(account).await?;
        if storage_deposit {
            self.storage_deposit(&account, Some(ONE_NEAR.saturating_div(100)), None)
                .await?;
        }
        self.mint(
            &account,
            None,
            token_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
        )
        .await?;
        Ok(account)
    }

    transaction! { fn storage_deposit(registration_only: Option<bool>) }
    transaction! { fn mint(token_ids: Vec<String>) }

    transaction! { fn nft_transfer(receiver_id: AccountId, token_id: TokenId, approval_id: Option<u32>, memo: Option<String>) }
    transaction! { fn nft_transfer_call(receiver_id: AccountId, token_id: TokenId, approval_id: Option<u32>, memo: Option<String>, msg: String) }
    read_only! { fn nft_token(token_id: TokenId) -> Option<Token> }
    read_only! { fn nft_metadata() -> ContractMetadata }

    read_only! { fn nft_total_supply() -> U128 }
    read_only! { fn nft_tokens(from_index: Option<U128>, limit: Option<u32>) -> Vec<Token> }
    read_only! { fn nft_supply_for_owner(account_id: AccountId) -> U128 }
    read_only! { fn nft_tokens_for_owner(account_id: AccountId, from_index: Option<U128>, limit: Option<u32>) -> Vec<Token> }

    transaction! { fn nft_approve(token_id: TokenId, account_id: AccountId, msg: Option<String>) }

    transaction! { fn nft_revoke(token_id: TokenId, account_id: AccountId) }

    transaction! { fn nft_revoke_all(token_id: TokenId) }

    read_only! { fn nft_is_approved(token_id: TokenId, approved_account_id: AccountId, approval_id: Option<ApprovalId>) -> bool }
}

#[tokio::test]
async fn create_and_mint() -> TestResult<()> {
    let s = setup(CONTRACT_171).await?;
    let alice = s.setup_account("alice", false, ["token_0"]).await?;
    let bob = s.setup_account("bob", false, ["token_1"]).await?;
    let charlie = s.setup_account("charlie", false, ["token_2"]).await?;

    let token_0 = s.nft_token("token_0").await?;
    let token_1 = s.nft_token("token_1").await?;
    let token_2 = s.nft_token("token_2").await?;
    let token_3 = s.nft_token("token_3").await?;

    // Verify minted tokens
    assert_eq!(
        token_0,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: alice.account_id().clone(),
            extensions_metadata: Default::default(),
        }),
    );
    assert_eq!(
        token_1,
        Some(Token {
            token_id: "token_1".to_string(),
            owner_id: bob.account_id().clone(),
            extensions_metadata: Default::default(),
        }),
    );
    assert_eq!(
        token_2,
        Some(Token {
            token_id: "token_2".to_string(),
            owner_id: charlie.account_id().clone(),
            extensions_metadata: Default::default(),
        }),
    );
    assert_eq!(token_3, None::<Token>);

    Ok(())
}

#[tokio::test]
async fn create_and_mint_with_metadata_and_enumeration() -> TestResult<()> {
    let s = setup(CONTRACT_FULL).await?;
    let alice = s.setup_account("alice", true, ["token_0"]).await?;
    let bob = s.setup_account("bob", true, ["token_1"]).await?;
    let charlie = s.setup_account("charlie", true, ["token_2"]).await?;

    let metadata = s.nft_metadata().await?;

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

    let token_0 = s.nft_token("token_0").await?;
    let token_1 = s.nft_token("token_1").await?;
    let token_2 = s.nft_token("token_2").await?;
    let token_3 = s.nft_token("token_3").await?;

    // Verify minted tokens
    assert_eq!(
        token_0,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: alice.account_id().clone(),
            extensions_metadata: [
                ("metadata".to_string(), token_meta("token_0")),
                ("approved_account_ids".to_string(), json!({})),
                ("funky_data".to_string(), json!({"funky": "data"})),
            ]
            .into(),
        }),
    );
    assert_eq!(
        token_1,
        Some(Token {
            token_id: "token_1".to_string(),
            owner_id: bob.account_id().clone(),
            extensions_metadata: [
                ("metadata".to_string(), token_meta("token_1")),
                ("approved_account_ids".to_string(), json!({})),
                ("funky_data".to_string(), json!({"funky": "data"})),
            ]
            .into(),
        }),
    );
    assert_eq!(
        token_2,
        Some(Token {
            token_id: "token_2".to_string(),
            owner_id: charlie.account_id().clone(),
            extensions_metadata: [
                ("metadata".to_string(), token_meta("token_2")),
                ("approved_account_ids".to_string(), json!({})),
                ("funky_data".to_string(), json!({"funky": "data"})),
            ]
            .into(),
        }),
    );
    assert_eq!(token_3, None::<Token>);

    // indeterminate order, so hashmap for equality instead of vec

    let all_tokens_enumeration = s
        .nft_tokens(None, None)
        .await?
        .into_iter()
        .map(|token| (token.token_id.clone(), token))
        .collect::<HashMap<_, _>>();
    let all_tokens_enumeration_limit = s
        .nft_tokens(Some(U128(0)), Some(100))
        .await?
        .into_iter()
        .map(|token| (token.token_id.clone(), token))
        .collect::<HashMap<_, _>>();
    let alice_supply = s.nft_supply_for_owner(alice.account_id()).await?;
    let alice_tokens_all = s
        .nft_tokens_for_owner(alice.account_id(), None, Some(100))
        .await?;
    let alice_tokens_offset = s
        .nft_tokens_for_owner(alice.account_id(), Some(U128(1)), None)
        .await?;
    let nonexistent_account_tokens = s
        .nft_tokens_for_owner(
            "0000000000000000000000000000000000000000000000000000000000000000"
                .parse::<AccountId>()
                .unwrap(),
            Some(U128(1)),
            None,
        )
        .await?;

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

    Ok(())
}

#[tokio::test]
async fn transfer_success() -> TestResult<()> {
    let s = setup(CONTRACT_171).await?;
    let alice = s.setup_account("alice", false, ["token_0"]).await?;
    let bob = s.setup_account("bob", false, ["token_1"]).await?;
    let charlie = s.setup_account("charlie", false, ["token_2"]).await?;

    let result = s
        .nft_transfer(&alice, Y, bob.account_id(), "token_0", None, None)
        .await?;

    assert_eq!(
        result.logs(),
        vec![
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                authorized_id: None,
                memo: None,
                token_ids: vec!["token_0".into()],
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
        ],
    );

    let token_0 = s.nft_token("token_0").await?;
    let token_1 = s.nft_token("token_1").await?;
    let token_2 = s.nft_token("token_2").await?;

    assert_eq!(
        token_0,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: bob.account_id().clone(),
            extensions_metadata: Default::default(),
        }),
    );
    assert_eq!(
        token_1,
        Some(Token {
            token_id: "token_1".to_string(),
            owner_id: bob.account_id().clone(),
            extensions_metadata: Default::default(),
        }),
    );
    assert_eq!(
        token_2,
        Some(Token {
            token_id: "token_2".to_string(),
            owner_id: charlie.account_id().clone(),
            extensions_metadata: Default::default(),
        }),
    );

    Ok(())
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Requires attached deposit of exactly 1 yoctoNEAR"]
async fn transfer_fail_no_deposit_full() {
    transfer_fail_no_deposit(CONTRACT_FULL, true).await.unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Requires attached deposit of exactly 1 yoctoNEAR"]
async fn transfer_fail_no_deposit_171() {
    transfer_fail_no_deposit(CONTRACT_171, false).await.unwrap();
}

async fn transfer_fail_no_deposit(wasm: &str, storage_deposit: bool) -> TestResult<()> {
    let s = setup(wasm).await?;
    let alice = s
        .setup_account("alice", storage_deposit, ["token_0"])
        .await?;
    let bob = s.setup_account("bob", storage_deposit, ["token_1"]).await?;

    s.nft_transfer(&alice, None, bob.account_id(), "token_0", None, None)
        .await?;

    Ok(())
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Token `token_5` does not exist"]
async fn transfer_fail_token_dne_full() {
    transfer_fail_token_dne(CONTRACT_FULL, true).await.unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Token `token_5` does not exist"]
async fn transfer_fail_token_dne_171() {
    transfer_fail_token_dne(CONTRACT_171, false).await.unwrap();
}

async fn transfer_fail_token_dne(wasm: &str, storage_deposit: bool) -> TestResult<()> {
    let s = setup(wasm).await?;
    let alice = s
        .setup_account("alice", storage_deposit, ["token_0"])
        .await?;
    let bob = s.setup_account("bob", storage_deposit, ["token_1"]).await?;

    s.nft_transfer(&alice, Y, bob.account_id(), "token_5", None, None)
        .await?;

    Ok(())
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Token `token_2` is owned by `charlie` instead of expected `alice`"]
async fn transfer_fail_not_owner_full() {
    transfer_fail_not_owner(CONTRACT_FULL, true).await.unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Token `token_2` is owned by `charlie` instead of expected `alice`"]
async fn transfer_fail_not_owner_171() {
    transfer_fail_not_owner(CONTRACT_171, false).await.unwrap();
}

async fn transfer_fail_not_owner(wasm: &str, storage_deposit: bool) -> TestResult<()> {
    let s = setup(wasm).await?;
    let alice = s
        .setup_account("alice", storage_deposit, ["token_0"])
        .await?;
    let bob = s.setup_account("bob", storage_deposit, ["token_1"]).await?;
    let _charlie = s
        .setup_account("charlie", storage_deposit, ["token_2"])
        .await?;

    s.nft_transfer(&alice, Y, bob.account_id(), "token_2", None, None)
        .await?;

    Ok(())
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Receiver must be different from current owner `alice` to transfer token `token_0`"]
async fn transfer_fail_reflexive_transfer_full() {
    transfer_fail_reflexive_transfer(CONTRACT_FULL, true)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Receiver must be different from current owner `alice` to transfer token `token_0`"]
async fn transfer_fail_reflexive_transfer_171() {
    transfer_fail_reflexive_transfer(CONTRACT_171, false)
        .await
        .unwrap();
}

async fn transfer_fail_reflexive_transfer(wasm: &str, storage_deposit: bool) -> TestResult<()> {
    let s = setup(wasm).await?;
    let alice = s
        .setup_account("alice", storage_deposit, ["token_0"])
        .await?;

    s.nft_transfer(&alice, Y, alice.account_id(), "token_0", None, None)
        .await?;

    Ok(())
}

#[tokio::test]
async fn transfer_call_success() -> TestResult<()> {
    let s = setup(CONTRACT_171).await?;
    let alice = s.setup_account("alice", false, ["token_0"]).await?;
    let bob = s.setup_account("bob", false, ["token_1"]).await?;

    s.handle
        .deploy(
            bob.account_id().clone(),
            &s.handle.load_wasm(CONTRACT_RECEIVER).await?,
            json!({}),
        )
        .await?;

    let result = s
        .nft_transfer_call(&alice, Y, bob.account_id(), "token_0", None, None, "")
        .await?;

    let logs = result.logs();

    assert_eq!(
        vec![
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".into()],
                authorized_id: None,
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                memo: None,
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
            format!(
                "Received token_0 from {} via {}",
                alice.account_id(),
                alice.account_id()
            ),
        ],
        logs
    );

    // not returned
    assert_eq!(
        s.nft_token("token_0").await?,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: bob.account_id().clone(),
            extensions_metadata: Default::default(),
        }),
    );

    Ok(())
}

#[tokio::test]
async fn transfer_call_return_success() -> TestResult<()> {
    let s = setup(CONTRACT_171).await?;
    let alice = s.setup_account("alice", false, ["token_0"]).await?;
    let bob = s.setup_account("bob", false, ["token_1"]).await?;

    s.handle
        .deploy(
            bob.account_id().clone(),
            &s.handle.load_wasm(CONTRACT_RECEIVER).await?,
            json!({}),
        )
        .await?;

    let result = s
        .nft_transfer_call(&alice, Y, bob.account_id(), "token_0", None, None, "return")
        .await?;

    let logs = result.logs();

    assert_eq!(
        vec![
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".into()],
                authorized_id: None,
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                memo: None,
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
            format!(
                "Received token_0 from {} via {}",
                alice.account_id(),
                alice.account_id()
            ),
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".into()],
                authorized_id: None,
                old_owner_id: bob.account_id().into(),
                new_owner_id: alice.account_id().into(),
                memo: None,
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
        ],
        logs
    );

    // returned
    assert_eq!(
        s.nft_token("token_0").await?,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: alice.account_id().clone(),
            extensions_metadata: Default::default(),
        }),
    );

    Ok(())
}

#[tokio::test]
async fn transfer_call_receiver_panic() -> TestResult<()> {
    let s = setup(CONTRACT_171).await?;
    let alice = s.setup_account("alice", false, ["token_0"]).await?;
    let bob = s.setup_account("bob", false, ["token_1"]).await?;
    s.handle
        .deploy(
            bob.account_id().clone(),
            &s.handle.load_wasm(CONTRACT_RECEIVER).await?,
            json!({}),
        )
        .await?;

    let result = s
        .nft_transfer_call(&alice, Y, bob.account_id(), "token_0", None, None, "panic")
        .await?;

    let logs = result.logs();

    assert_eq!(
        vec![
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".into()],
                authorized_id: None,
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                memo: None,
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
            format!(
                "Received token_0 from {} via {}",
                alice.account_id(),
                alice.account_id()
            ),
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".into()],
                authorized_id: None,
                old_owner_id: bob.account_id().into(),
                new_owner_id: alice.account_id().into(),
                memo: None,
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
        ],
        logs
    );

    // returned
    assert_eq!(
        s.nft_token("token_0").await?,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: alice.account_id().clone(),
            extensions_metadata: Default::default(),
        }),
    );

    Ok(())
}

#[tokio::test]
async fn transfer_call_receiver_send_return() -> TestResult<()> {
    let s = setup(CONTRACT_171).await?;
    let alice = s.setup_account("alice", false, ["token_0"]).await?;
    let bob = s.setup_account("bob", false, ["token_1"]).await?;
    let charlie = s.setup_account("charlie", false, ["token_2"]).await?;

    s.handle
        .deploy(
            bob.account_id().clone(),
            &s.handle.load_wasm(CONTRACT_RECEIVER).await?,
            json!({}),
        )
        .await?;

    let msg = format!("transfer:{}", charlie.account_id());
    let result = s
        .nft_transfer_call(&alice, Y, bob.account_id(), "token_0", None, None, msg)
        .await?;

    let logs = result.logs();

    println!("{logs:#?}");

    assert_eq!(
        vec![
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".into()],
                authorized_id: None,
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                memo: None,
            }])
            .to_event_string(),
            "after_nft_transfer(token_0)".to_string(),
            format!(
                "Received token_0 from {} via {}",
                alice.account_id(),
                alice.account_id()
            ),
            format!("Transferring token_0 to {}", charlie.account_id()),
            "before_nft_transfer(token_0)".to_string(),
            Nep171Event::NftTransfer(vec![NftTransferLog {
                token_ids: vec!["token_0".into()],
                authorized_id: None,
                old_owner_id: bob.account_id().into(),
                new_owner_id: charlie.account_id().into(),
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
        s.nft_token("token_0").await?,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: charlie.account_id().clone(),
            extensions_metadata: Default::default(),
        }),
    );

    Ok(())
}

#[tokio::test]
async fn transfer_approval_success() -> TestResult<()> {
    let s = setup(CONTRACT_FULL).await?;
    let alice = s.setup_account("alice", true, ["token_0"]).await?;
    let bob = s.setup_account("bob", true, ["token_1"]).await?;
    let charlie = s.setup_account("charlie", true, ["token_2"]).await?;

    s.nft_approve(&alice, Y, "token_0", bob.account_id(), None)
        .await?;

    let view_token = s.nft_token("token_0").await?;

    let expected_view_token = Token {
        token_id: "token_0".into(),
        owner_id: alice.account_id().clone(),
        extensions_metadata: [
            ("metadata".to_string(), token_meta("token_0")),
            (
                "approved_account_ids".to_string(),
                json!({
                    bob.account_id().to_string(): 0,
                }),
            ),
            ("funky_data".to_string(), json!({"funky": "data"})),
        ]
        .into(),
    };

    assert_eq!(view_token, Some(expected_view_token));

    let is_approved = s.nft_is_approved("token_0", bob.account_id(), None).await?;

    assert!(is_approved);

    s.nft_transfer(&bob, Y, charlie.account_id(), "token_0", Some(0), None)
        .await?;

    assert_eq!(
        s.nft_token("token_0").await?,
        Some(Token {
            token_id: "token_0".to_string(),
            owner_id: charlie.account_id().clone(),
            extensions_metadata: [
                ("metadata".to_string(), token_meta("token_0")),
                ("approved_account_ids".to_string(), json!({})),
                ("funky_data".to_string(), json!({"funky": "data"})),
            ]
            .into(),
        }),
    );

    Ok(())
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Sender `bob` does not have permission to transfer token `token_0`, owned by `alice`, with approval ID 0"]
async fn transfer_approval_unapproved_fail() {
    let s = setup(CONTRACT_FULL).await.unwrap();
    let alice = s.setup_account("alice", true, ["token_0"]).await.unwrap();
    let bob = s.setup_account("bob", true, ["token_1"]).await.unwrap();
    let charlie = s.setup_account("charlie", true, ["token_2"]).await.unwrap();
    let debbie = s.setup_account("debbie", true, ["token_3"]).await.unwrap();

    s.nft_approve(&alice, Y, "token_0", debbie.account_id(), None)
        .await
        .unwrap();

    let is_approved = s
        .nft_is_approved("token_0", bob.account_id(), None)
        .await
        .unwrap();

    assert!(!is_approved);

    s.nft_transfer(&bob, Y, charlie.account_id(), "token_0", Some(0), None)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Attached deposit must be greater than zero"]
async fn transfer_approval_no_deposit_fail() {
    let s = setup(CONTRACT_FULL).await.unwrap();
    let alice = s.setup_account("alice", true, ["token_0"]).await.unwrap();
    let bob = s.setup_account("bob", true, ["token_1"]).await.unwrap();

    s.nft_approve(&alice, None, "token_0", bob.account_id(), None)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Account bob is already approved for token token_0."]
async fn transfer_approval_double_approval_fail() {
    let s = setup(CONTRACT_FULL).await.unwrap();
    let alice = s.setup_account("alice", true, ["token_0"]).await.unwrap();
    let bob = s.setup_account("bob", true, ["token_1"]).await.unwrap();

    s.nft_approve(&alice, Y, "token_0", bob.account_id(), None)
        .await
        .unwrap();
    s.nft_approve(&alice, Y, "token_0", bob.account_id(), None)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Account `bob` is not authorized to manage approvals for token `token_0`"]
async fn transfer_approval_unauthorized_approval_fail() {
    let s = setup(CONTRACT_FULL).await.unwrap();
    let _alice = s.setup_account("alice", true, ["token_0"]).await.unwrap();
    let bob = s.setup_account("bob", true, ["token_1"]).await.unwrap();

    s.nft_approve(&bob, Y, "token_0", bob.account_id(), None)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Too many approvals for token token_0, maximum is 32."]
async fn transfer_approval_too_many_approvals_fail() {
    let s = setup(CONTRACT_FULL).await.unwrap();
    let alice = s.setup_account("alice", true, ["token_0"]).await.unwrap();
    let bob = s.setup_account("bob", true, ["token_1"]).await.unwrap();

    for i in 0..32 {
        let id: AccountId = format!("account_{i}").parse().unwrap();
        s.nft_approve(&alice, Y, "token_0", id, None).await.unwrap();
    }

    s.nft_approve(&alice, Y, "token_0", bob.account_id(), None)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Sender `bob` does not have permission to transfer token `token_0`, owned by `alice`, with approval ID 1"]
async fn transfer_approval_approved_but_wrong_approval_id_fail() {
    let s = setup(CONTRACT_FULL).await.unwrap();
    let alice = s.setup_account("alice", true, ["token_0"]).await.unwrap();
    let bob = s.setup_account("bob", true, ["token_1"]).await.unwrap();
    let charlie = s.setup_account("charlie", true, ["token_2"]).await.unwrap();

    s.nft_approve(&alice, Y, "token_0", bob.account_id(), None)
        .await
        .unwrap();

    s.nft_transfer(&bob, Y, charlie.account_id(), "token_0", Some(1), None)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic = "Account this_account_is_not_registered.near is not registered"]
async fn transfer_fail_not_registered_nep145() {
    let s = setup(CONTRACT_FULL).await.unwrap();
    let alice = s.setup_account("alice", true, ["token_0"]).await.unwrap();

    let id: AccountId = "this_account_is_not_registered.near".parse().unwrap();
    s.nft_transfer(&alice, Y, id, "token_0", None, None)
        .await
        .unwrap();
}
