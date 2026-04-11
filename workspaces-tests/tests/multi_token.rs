use near_api::{Account, Contract};
use near_sdk::{
    AccountId,
    json_types::{Base64VecU8, U128},
    serde_json::json,
};
use near_sdk_contract_tools::{mt::*, standard::nep297::Event};
use pretty_assertions::assert_eq;
use testresult::TestResult;
use workspaces_tests::{Handle, ONE_NEAR, ONE_YOCTO, Y, read_only, transaction};

struct Setup {
    pub handle: Handle,
    pub contract: Contract,
}

/// Setup for individual tests
async fn setup() -> TestResult<Setup> {
    let handle = Handle::new().await;
    let contract = handle
        .make_contract("multi_token", "multi_token", json!({}))
        .await?;

    Ok(Setup { contract, handle })
}

impl Setup {
    async fn setup_account(&self, name: impl Into<String>, amount: u128) -> TestResult<Account> {
        let account = self.handle.make_account(name).await?;
        self.storage_deposit(&account, Some(ONE_NEAR.saturating_div(100)))
            .await?;
        self.mint(&account, None, "my_token", amount).await?;
        Ok(account)
    }

    transaction! { fn storage_deposit() }
    read_only! { fn storage_balance_of(account_id: AccountId) -> Option<StorageBalance> }
    transaction! { fn mint(token_id: String, amount: U128) }
    transaction! { fn use_storage(blob: Base64VecU8) }
    transaction! { fn create_base_meta(base_metadata: BaseMetadata) }
    transaction! { fn remove_base_meta(base_metadata_id: BaseMetadataId) }
    transaction! { fn set_token_meta(token_id: TokenId, base_metadata_id: BaseMetadataId, token_metadata: TokenMetadata) }
    transaction! { fn mt_transfer( receiver_id: AccountId, token_id: TokenId, amount: U128, approval: Option<MtTransferApproval>, memo: Option<String>) }
    transaction! { fn mt_batch_transfer( receiver_id: AccountId, token_ids: Vec<TokenId>, amounts: Vec<U128>, approvals: Option<Vec<Option<MtTransferApproval>>>, memo: Option<String>) }
    transaction! { fn mt_transfer_call( receiver_id: AccountId, token_id: TokenId, amount: U128, approval: Option<MtTransferApproval>, memo: Option<String>, msg: String) }
    transaction! { fn mt_batch_transfer_call( receiver_id: AccountId, token_ids: Vec<TokenId>, amounts: Vec<U128>, approvals: Option<Vec<Option<MtTransferApproval>>>, memo: Option<String>, msg: String) }
    read_only! { fn mt_token(token_ids: Vec<TokenId>) -> Vec<Option<Token>> }
    read_only! { fn mt_balance_of(account_id: AccountId, token_id: TokenId) -> U128 }
    read_only! { fn mt_batch_balance_of(account_id: AccountId, token_ids: Vec<TokenId>) -> Vec<U128> }
    read_only! { fn mt_supply(token_id: TokenId) -> Option<U128> }
    read_only! { fn mt_batch_supply(token_ids: Vec<TokenId>) -> Vec<Option<U128>> }

    read_only! { fn mt_metadata_contract() -> ContractMetadata }
    read_only! { fn mt_metadata_token_all(token_ids: Vec<TokenId>) -> Vec<TokenMetadataAll> }
    read_only! { fn mt_metadata_token_by_token_id(token_ids: Vec<TokenId>) -> Vec<TokenMetadata> }
    read_only! { fn mt_metadata_base_by_token_id(token_ids: Vec<TokenId>) -> Vec<BaseMetadata> }
    read_only! { fn mt_metadata_base_by_metadata_id( base_metadata_ids: Vec<BaseMetadataId>) -> Vec<BaseMetadata> }
}

#[tokio::test]
async fn start_empty() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.handle.make_account("alice").await?;
    let bob = s.handle.make_account("bob").await?;
    let charlie = s.handle.make_account("charlie").await?;

    // All accounts must start with 0 balance
    assert_eq!(s.mt_balance_of(alice.account_id(), "my_token").await?.0, 0);
    assert_eq!(s.mt_balance_of(bob.account_id(), "my_token").await?.0, 0);
    assert_eq!(
        s.mt_balance_of(charlie.account_id(), "my_token").await?.0,
        0
    );

    Ok(())
}

#[tokio::test]
async fn contract_metadata() -> TestResult<()> {
    let s = setup().await?;

    let contract_metadata = s.mt_metadata_contract().await?;

    assert_eq!(
        contract_metadata,
        ContractMetadata {
            spec: ContractMetadata::SPEC.to_string(),
            name: "My MultiToken".to_string()
        },
    );

    Ok(())
}

#[tokio::test]
async fn token_metadata() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;

    let base_metadata = BaseMetadata::new("Base Metadata", "base_0");

    s.create_base_meta(&alice, None, base_metadata.clone())
        .await?;

    let token_metadata = TokenMetadata::new()
        .title("Token Title")
        .description("Token Description");

    s.set_token_meta(&alice, None, "my_token", "base_0", token_metadata.clone())
        .await?;

    let metadata_token_all = s.mt_metadata_token_all(["my_token".to_string()]).await?;
    assert_eq!(
        metadata_token_all,
        vec![TokenMetadataAll {
            base: base_metadata.clone(),
            token: token_metadata.clone()
        }],
    );

    let metadata_token_by_token_id = s
        .mt_metadata_token_by_token_id(["my_token".to_string()])
        .await?;
    assert_eq!(metadata_token_by_token_id, vec![token_metadata.clone()]);

    let metadata_base_by_token_id = s
        .mt_metadata_base_by_token_id(["my_token".to_string()])
        .await?;
    assert_eq!(metadata_base_by_token_id, vec![base_metadata.clone()]);

    let metadata_base_by_metadata_id = s
        .mt_metadata_base_by_metadata_id(["base_0".to_string()])
        .await?;
    assert_eq!(metadata_base_by_metadata_id, vec![base_metadata.clone()]);

    Ok(())
}

#[tokio::test]
async fn mint() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    // Verify issued balances
    assert_eq!(
        s.mt_balance_of(alice.account_id(), "my_token").await?.0,
        1000
    );
    assert_eq!(s.mt_balance_of(bob.account_id(), "my_token").await?.0, 100);
    assert_eq!(
        s.mt_balance_of(charlie.account_id(), "my_token").await?.0,
        10
    );

    Ok(())
}

#[tokio::test]
async fn transfer_normal() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    s.mt_transfer(&alice, Y, bob.account_id(), "my_token", 10, None, None)
        .await?;
    assert_eq!(
        s.mt_balance_of(alice.account_id(), "my_token").await?.0,
        990
    );
    assert_eq!(s.mt_balance_of(bob.account_id(), "my_token").await?.0, 110);
    assert_eq!(
        s.mt_balance_of(charlie.account_id(), "my_token").await?.0,
        10
    );

    Ok(())
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Token ID token_dne does not exist."]
async fn transfer_token_id_dne() {
    let s = setup().await.unwrap();

    let alice = s.setup_account("alice", 1000).await.unwrap();
    let bob = s.setup_account("bob", 100).await.unwrap();

    s.mt_transfer(&alice, Y, bob.account_id(), "token_dne", 1, None, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn batch_transfer_normal() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    s.mint(&alice, None, "my_token_2", 1000).await?;
    let token_ids = ["my_token".to_string(), "my_token_2".to_string()];
    s.mt_batch_transfer(
        &alice,
        Y,
        bob.account_id(),
        token_ids.clone(),
        [U128(3), U128(33)],
        None,
        None,
    )
    .await?;

    assert_eq!(
        s.mt_batch_balance_of(alice.account_id(), token_ids.clone())
            .await?,
        [U128(997), U128(967)],
    );
    assert_eq!(
        s.mt_batch_balance_of(bob.account_id(), token_ids.clone())
            .await?,
        [U128(103), U128(33)],
    );
    assert_eq!(
        s.mt_batch_balance_of(charlie.account_id(), token_ids.clone())
            .await?,
        [U128(10), U128(0)],
    );

    Ok(())
}

#[tokio::test]
async fn transfer_zero() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    s.mt_transfer(&alice, Y, bob.account_id(), "my_token", 0, None, None)
        .await?;
    assert_eq!(
        s.mt_balance_of(alice.account_id(), "my_token").await?.0,
        1000
    );
    assert_eq!(s.mt_balance_of(bob.account_id(), "my_token").await?.0, 100);
    assert_eq!(
        s.mt_balance_of(charlie.account_id(), "my_token").await?.0,
        10
    );

    Ok(())
}

#[tokio::test]
#[should_panic(expected = "invalid digit found in string")]
async fn transfer_negative() {
    let s = setup().await.unwrap();

    let alice = s.setup_account("alice", 1000).await.unwrap();
    let bob = s.setup_account("bob", 100).await.unwrap();

    s.contract
        .call_function(
            "mt_transfer",
            json!({
                "receiver_id": bob.account_id(),
                "token_id": "my_token",
                "amount": "-10",
            }),
        )
        .transaction()
        .deposit(ONE_YOCTO)
        .with_signer(alice.account_id().clone(), s.handle.default_signer())
        .send_to(&s.handle.network)
        .await
        .unwrap()
        .assert_success();
}

#[tokio::test]
#[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
async fn transfer_no_deposit() {
    let s = setup().await.unwrap();

    let alice = s.setup_account("alice", 1000).await.unwrap();
    let bob = s.setup_account("bob", 100).await.unwrap();

    s.contract
        .call_function(
            "mt_transfer",
            json!({
                "receiver_id": bob.account_id(),
                "token_id": "my_token",
                "amount": "10",
            }),
        )
        .transaction()
        .with_signer(alice.account_id().clone(), s.handle.default_signer())
        .send_to(&s.handle.network)
        .await
        .unwrap()
        .assert_success();
}

#[tokio::test]
#[should_panic(expected = "Balance of the sender is insufficient")]
async fn transfer_more_than_balance() {
    let s = setup().await.unwrap();

    let alice = s.setup_account("alice", 1000).await.unwrap();
    let bob = s.setup_account("bob", 100).await.unwrap();

    s.mt_transfer(&alice, Y, bob.account_id(), "my_token", 1000000, None, None)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "would overflow u128")]
async fn transfer_overflow_u128() {
    let s = setup().await.unwrap();

    let alice = s.setup_account("alice", u128::MAX / 2).await.unwrap();
    let _bob = s.setup_account("bob", u128::MAX / 2).await.unwrap();

    s.mint(&alice, None, "my_token", 2).await.unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Account charlie is not registered"]
async fn transfer_fail_not_registered() {
    let s = setup().await.unwrap();

    let alice = s.setup_account("alice", 1000).await.unwrap();
    let charlie = s.handle.make_account("charlie").await.unwrap();

    s.mt_transfer(&alice, Y, charlie.account_id(), "my_token", 10, None, None)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Storage lock error: Account alice has insufficient balance: 0.010 NEAR available, but attempted to use 0.101 NEAR"]
async fn fail_run_out_of_space() {
    let s = setup().await.unwrap();

    let alice = s.setup_account("alice", 1000).await.unwrap();

    s.use_storage(&alice, None, vec![1u8; 10000]).await.unwrap();
}

#[tokio::test]
async fn transfer_call_normal() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    s.handle
        .deploy(
            bob.account_id().clone(),
            &s.handle.load_wasm("multi_token_receiver").await?,
            json!({}),
        )
        .await?;

    let result = s
        .mt_transfer_call(
            &alice,
            Y,
            bob.account_id().clone(),
            "my_token",
            10,
            None,
            None,
            "", // keep all of the tokens
        )
        .await?;

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep245Event::MtTransfer(vec![MtTransferData {
                authorized_id: None,
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                memo: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.account_id(),
                alice.account_id(),
            ),
        ]
    );

    assert_eq!(
        s.mt_balance_of(alice.account_id(), "my_token").await?.0,
        990
    );
    assert_eq!(s.mt_balance_of(bob.account_id(), "my_token").await?.0, 110);
    assert_eq!(
        s.mt_balance_of(charlie.account_id(), "my_token").await?.0,
        10
    );

    Ok(())
}

#[tokio::test]
async fn batch_transfer_call_normal() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    s.handle
        .deploy(
            bob.account_id().clone(),
            &s.handle.load_wasm("multi_token_receiver").await?,
            json!({}),
        )
        .await?;

    s.mint(&alice, None, "my_token_2", 1000).await?;

    let token_ids = ["my_token".to_string(), "my_token_2".to_string()];
    let result = s
        .mt_batch_transfer_call(
            &alice,
            Y,
            bob.account_id(),
            token_ids.clone(),
            [U128(10), U128(20)],
            None,
            None,
            "", // keep all of the tokens
        )
        .await?;

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep245Event::MtTransfer(vec![MtTransferData {
                authorized_id: None,
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                memo: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![10.into(), 20.into()],
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.account_id(),
                alice.account_id(),
            ),
            format!(
                "Received 20 of my_token_2 from {} via {}",
                alice.account_id(),
                alice.account_id(),
            ),
        ]
    );

    assert_eq!(
        s.mt_batch_balance_of(alice.account_id(), &token_ids)
            .await?,
        [U128(990), U128(980)],
    );
    assert_eq!(
        s.mt_batch_balance_of(bob.account_id(), &token_ids).await?,
        [U128(110), U128(20)],
    );
    assert_eq!(
        s.mt_batch_balance_of(charlie.account_id(), &token_ids)
            .await?,
        [U128(10), U128(0)],
    );

    Ok(())
}

#[tokio::test]
async fn transfer_call_return() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    s.handle
        .deploy(
            bob.account_id().clone(),
            &s.handle.load_wasm("multi_token_receiver").await?,
            json!({}),
        )
        .await?;

    let result = s
        .mt_transfer_call(
            &alice,
            Y,
            bob.account_id(),
            "my_token",
            10,
            None,
            None,
            "return", // return all of the tokens
        )
        .await?;

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                memo: None,
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.account_id(),
                alice.account_id(),
            ),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.account_id().into(),
                new_owner_id: alice.account_id().into(),
                memo: None,
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(
        s.mt_balance_of(alice.account_id(), "my_token").await?.0,
        1000
    );
    assert_eq!(s.mt_balance_of(bob.account_id(), "my_token").await?.0, 100);
    assert_eq!(
        s.mt_balance_of(charlie.account_id(), "my_token").await?.0,
        10
    );

    Ok(())
}

#[tokio::test]
async fn batch_transfer_call_return() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    s.handle
        .deploy(
            bob.account_id().clone(),
            &s.handle.load_wasm("multi_token_receiver").await?,
            json!({}),
        )
        .await?;

    s.mint(&alice, None, "my_token_2", 1000).await?;

    let token_ids = ["my_token".to_string(), "my_token_2".to_string()];
    let result = s
        .mt_batch_transfer_call(
            &alice,
            Y,
            bob.account_id(),
            &token_ids,
            [U128(10), U128(20)],
            None,
            None,
            r#"return:["1","2"]"#,
        )
        .await?;

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                memo: None,
                authorized_id: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![10.into(), 20.into()],
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.account_id(),
                alice.account_id(),
            ),
            format!(
                "Received 20 of my_token_2 from {} via {}",
                alice.account_id(),
                alice.account_id(),
            ),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.account_id().into(),
                new_owner_id: alice.account_id().into(),
                memo: None,
                authorized_id: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![1.into(), 2.into()],
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(
        s.mt_batch_balance_of(alice.account_id(), &token_ids)
            .await?,
        [U128(991), U128(982)],
    );
    assert_eq!(
        s.mt_batch_balance_of(bob.account_id(), &token_ids).await?,
        [U128(109), U128(18)],
    );
    assert_eq!(
        s.mt_batch_balance_of(charlie.account_id(), &token_ids)
            .await?,
        [U128(10), U128(0)],
    );

    Ok(())
}

#[tokio::test]
async fn transfer_call_inner_transfer() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    s.handle
        .deploy(
            bob.account_id().clone(),
            &s.handle.load_wasm("multi_token_receiver").await?,
            json!({}),
        )
        .await?;

    let msg = format!("transfer:{}", charlie.account_id());
    let result = s
        .mt_transfer_call(&alice, Y, bob.account_id(), "my_token", 10, None, None, msg)
        .await?;

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
                memo: None,
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.account_id(),
                alice.account_id(),
            ),
            format!("Transferring all to {}", charlie.account_id()),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.account_id().into(),
                new_owner_id: charlie.account_id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
                memo: None,
            }])
            .to_event_string(),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.account_id().into(),
                new_owner_id: alice.account_id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(
        s.mt_balance_of(alice.account_id(), "my_token").await?.0,
        1000
    );
    assert_eq!(s.mt_balance_of(bob.account_id(), "my_token").await?.0, 90);
    assert_eq!(
        s.mt_balance_of(charlie.account_id(), "my_token").await?.0,
        20
    );

    Ok(())
}

#[tokio::test]
async fn batch_transfer_call_inner_transfer() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    s.handle
        .deploy(
            bob.account_id().clone(),
            &s.handle.load_wasm("multi_token_receiver").await?,
            json!({}),
        )
        .await?;

    s.mint(&alice, None, "my_token_2", 1000).await?;

    let token_ids = ["my_token".to_string(), "my_token_2".to_string()];
    let msg = format!("transfer:{}", charlie.account_id());
    let result = s
        .mt_batch_transfer_call(
            &alice,
            Y,
            bob.account_id(),
            &token_ids,
            [U128(10), U128(20)],
            None,
            None,
            msg,
        )
        .await?;

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![10.into(), 20.into()],
                memo: None,
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.account_id(),
                alice.account_id(),
            ),
            format!(
                "Received 20 of my_token_2 from {} via {}",
                alice.account_id(),
                alice.account_id(),
            ),
            format!("Transferring all to {}", charlie.account_id()),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.account_id().into(),
                new_owner_id: charlie.account_id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![10.into(), 20.into()],
                memo: None,
            }])
            .to_event_string(),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.account_id().into(),
                new_owner_id: alice.account_id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(
        s.mt_batch_balance_of(alice.account_id(), &token_ids)
            .await?,
        [U128(1000), U128(1000 - 20)],
    );
    assert_eq!(
        s.mt_batch_balance_of(bob.account_id(), &token_ids).await?,
        [U128(100 - 10), U128(0)],
    );
    assert_eq!(
        s.mt_batch_balance_of(charlie.account_id(), &token_ids)
            .await?,
        [U128(20), U128(20)],
    );

    Ok(())
}

#[tokio::test]
async fn transfer_call_inner_panic() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    s.handle
        .deploy(
            bob.account_id().clone(),
            &s.handle.load_wasm("multi_token_receiver").await?,
            json!({}),
        )
        .await?;

    let result = s
        .mt_transfer_call(
            &alice,
            Y,
            bob.account_id(),
            "my_token",
            10,
            None,
            None,
            "panic",
        )
        .await?;

    let inner_outcome = result.outcomes().to_vec()[2];

    assert!(inner_outcome.is_failure());

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
                memo: None,
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.account_id(),
                alice.account_id(),
            ),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.account_id().into(),
                new_owner_id: alice.account_id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into()],
                amounts: vec![10.into()],
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(
        s.mt_balance_of(alice.account_id(), "my_token").await?.0,
        1000
    );
    assert_eq!(s.mt_balance_of(bob.account_id(), "my_token").await?.0, 100);
    assert_eq!(
        s.mt_balance_of(charlie.account_id(), "my_token").await?.0,
        10
    );

    Ok(())
}

#[tokio::test]
async fn batch_transfer_call_inner_panic() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    s.handle
        .deploy(
            bob.account_id().clone(),
            &s.handle.load_wasm("multi_token_receiver").await?,
            json!({}),
        )
        .await?;

    s.mint(&alice, None, "my_token_2", 1000).await?;

    let token_ids = ["my_token".to_string(), "my_token_2".to_string()];
    let result = s
        .mt_batch_transfer_call(
            &alice,
            Y,
            bob.account_id(),
            &token_ids,
            [U128(10), U128(20)],
            None,
            None,
            "panic",
        )
        .await?;

    let inner_outcome = result.outcomes().to_vec()[2];

    assert!(inner_outcome.is_failure());

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![10.into(), 20.into()],
                memo: None,
            }])
            .to_event_string(),
            format!(
                "Received 10 of my_token from {} via {}",
                alice.account_id(),
                alice.account_id(),
            ),
            format!(
                "Received 20 of my_token_2 from {} via {}",
                alice.account_id(),
                alice.account_id(),
            ),
            Nep245Event::MtTransfer(vec![MtTransferData {
                old_owner_id: bob.account_id().into(),
                new_owner_id: alice.account_id().into(),
                authorized_id: None,
                token_ids: vec!["my_token".into(), "my_token_2".into()],
                amounts: vec![10.into(), 20.into()],
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(
        s.mt_batch_balance_of(alice.account_id(), &token_ids)
            .await?,
        [U128(1000), U128(1000)],
    );
    assert_eq!(
        s.mt_batch_balance_of(bob.account_id(), &token_ids).await?,
        [U128(100), U128(0)],
    );
    assert_eq!(
        s.mt_batch_balance_of(charlie.account_id(), &token_ids)
            .await?,
        [U128(10), U128(0)],
    );

    Ok(())
}
