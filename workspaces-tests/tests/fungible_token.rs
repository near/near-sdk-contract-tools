use near_api::{Account, Contract};
use near_sdk::{
    AccountId,
    json_types::{Base64VecU8, U128},
    serde_json::json,
};
use near_sdk_contract_tools::{
    ft::nep141::FtBurnData,
    nft::StorageBalance,
    standard::{
        nep141::{FtTransferData, Nep141Event},
        nep297::Event,
    },
};
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
        .make_contract("fungible_token", "fungible_token", json!({}))
        .await?;

    Ok(Setup { contract, handle })
}

impl Setup {
    async fn setup_account(&self, name: impl Into<String>, amount: u128) -> TestResult<Account> {
        let account = self.handle.make_account(name).await?;
        self.storage_deposit(&account, Some(ONE_NEAR.saturating_div(100)))
            .await?;
        self.mint(&account, None, amount).await?;
        Ok(account)
    }

    transaction! { fn storage_deposit() }
    read_only! { fn ft_balance_of(account_id: AccountId) -> U128 }
    transaction! { fn mint(amount: U128) }
    transaction! { fn ft_transfer(receiver_id: AccountId, amount: U128) }
    read_only! { fn storage_balance_of(account_id: AccountId) -> Option<StorageBalance> }
    read_only! { fn total_supply() -> U128 }
    transaction! { fn use_storage(blob: Base64VecU8) }
    transaction! { fn ft_transfer_call(receiver_id: AccountId, amount: U128, msg: String) }
    transaction! { fn storage_unregister(force: Option<bool>) }
}

#[tokio::test]
async fn start_empty() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.handle.make_account("alice").await?;
    let bob = s.handle.make_account("bob").await?;

    // All accounts must start with 0 balance
    assert_eq!(s.ft_balance_of(alice.account_id()).await?.0, 0);
    assert_eq!(s.ft_balance_of(bob.account_id()).await?.0, 0);

    Ok(())
}

#[tokio::test]
async fn mint() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    // Verify issued balances
    assert_eq!(s.ft_balance_of(alice.account_id()).await?.0, 1000);
    assert_eq!(s.ft_balance_of(bob.account_id()).await?.0, 100);
    assert_eq!(s.ft_balance_of(charlie.account_id()).await?.0, 10);

    Ok(())
}

#[tokio::test]
async fn transfer_normal() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    s.ft_transfer(&alice, Y, bob.account_id(), 10).await?;

    assert_eq!(s.ft_balance_of(alice.account_id()).await?.0, 990);
    assert_eq!(s.ft_balance_of(bob.account_id()).await?.0, 110);
    assert_eq!(s.ft_balance_of(charlie.account_id()).await?.0, 10);

    Ok(())
}

#[tokio::test]
async fn transfer_zero() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;
    let bob = s.setup_account("bob", 100).await?;
    let charlie = s.setup_account("charlie", 10).await?;

    let _ = s.ft_transfer(&alice, Y, bob.account_id(), 0).await;

    assert_eq!(s.ft_balance_of(alice.account_id()).await?.0, 1000);
    assert_eq!(s.ft_balance_of(bob.account_id()).await?.0, 100);
    assert_eq!(s.ft_balance_of(charlie.account_id()).await?.0, 10);

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
            "ft_transfer",
            json!({
                "receiver_id": bob.account_id(),
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

    s.ft_transfer(&alice, None, bob.account_id(), 10)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "Balance of the sender is insufficient")]
async fn transfer_more_than_balance() {
    let s = setup().await.unwrap();

    let alice = s.setup_account("alice", 1000).await.unwrap();
    let bob = s.setup_account("bob", 100).await.unwrap();

    s.ft_transfer(&alice, Y, bob.account_id(), 1000000)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(expected = "TotalSupplyOverflowError")]
async fn transfer_overflow_u128() {
    let s = setup().await.unwrap();

    let alice = s.setup_account("alice", u128::MAX / 2).await.unwrap();
    let _bob = s.setup_account("bob", u128::MAX / 2).await.unwrap();

    s.mint(&alice, None, u128::MAX / 2).await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "Account charlie is not registered")]
async fn transfer_fail_not_registered() {
    let s = setup().await.unwrap();

    let alice = s.setup_account("alice", 1000).await.unwrap();
    let charlie = s.handle.make_account("charlie").await.unwrap();

    s.ft_transfer(&alice, Y, charlie.account_id(), 10)
        .await
        .unwrap();
}

#[tokio::test]
#[should_panic(
    expected = "Storage lock error: Account alice has insufficient balance: 0.009 NEAR available, but attempted to use 0.101 NEAR"
)]
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
            &s.handle.load_wasm("fungible_token_receiver").await?,
            json!({}),
        )
        .await?;

    let result = s
        .ft_transfer_call(&alice, Y, bob.account_id(), 10, "".to_string())
        .await?;

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
            format!("Received 10 from {}", alice.account_id()),
        ]
    );

    assert_eq!(s.ft_balance_of(alice.account_id()).await?.0, 990);
    assert_eq!(s.ft_balance_of(bob.account_id()).await?.0, 110);
    assert_eq!(s.ft_balance_of(charlie.account_id()).await?.0, 10);

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
            &s.handle.load_wasm("fungible_token_receiver").await?,
            json!({}),
        )
        .await?;

    let result = s
        .ft_transfer_call(&alice, Y, bob.account_id(), 10, "return")
        .await?;

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
            format!("Received 10 from {}", alice.account_id()),
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: bob.account_id().into(),
                new_owner_id: alice.account_id().into(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(s.ft_balance_of(alice.account_id()).await?.0, 1000);
    assert_eq!(s.ft_balance_of(bob.account_id()).await?.0, 100);
    assert_eq!(s.ft_balance_of(charlie.account_id()).await?.0, 10);

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
            &s.handle.load_wasm("fungible_token_receiver").await?,
            json!({}),
        )
        .await?;

    let msg = format!("transfer:{}", charlie.account_id());
    let result = s
        .ft_transfer_call(&alice, Y, bob.account_id(), 10, msg)
        .await?;

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
            format!("Received 10 from {}", alice.account_id()),
            format!("Transferring 10 to {}", charlie.account_id()),
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: bob.account_id().into(),
                new_owner_id: charlie.account_id().into(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: bob.account_id().into(),
                new_owner_id: alice.account_id().into(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(s.ft_balance_of(alice.account_id()).await?.0, 1000);
    assert_eq!(s.ft_balance_of(bob.account_id()).await?.0, 90);
    assert_eq!(s.ft_balance_of(charlie.account_id()).await?.0, 20);

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
            &s.handle.load_wasm("fungible_token_receiver").await?,
            json!({}),
        )
        .await?;

    let result = s
        .ft_transfer_call(&alice, Y, bob.account_id(), 10, "panic")
        .await?;

    let inner_outcome = result.outcomes().to_vec()[2];

    assert!(inner_outcome.is_failure());

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: alice.account_id().into(),
                new_owner_id: bob.account_id().into(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
            format!("Received 10 from {}", alice.account_id()),
            Nep141Event::FtTransfer(vec![FtTransferData {
                old_owner_id: bob.account_id().into(),
                new_owner_id: alice.account_id().into(),
                amount: U128(10),
                memo: None,
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(s.ft_balance_of(alice.account_id()).await?.0, 1000);
    assert_eq!(s.ft_balance_of(bob.account_id()).await?.0, 100);
    assert_eq!(s.ft_balance_of(charlie.account_id()).await?.0, 10);

    Ok(())
}

#[tokio::test]
async fn force_unregister() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice", 1000).await?;

    let result = s.storage_unregister(&alice, Y, true).await?;

    assert_eq!(
        result.logs().to_vec(),
        vec![
            Nep141Event::FtBurn(vec![FtBurnData {
                owner_id: alice.account_id().into(),
                amount: U128(1000),
                memo: Some("storage forced unregistration".into()),
            }])
            .to_event_string(),
        ]
    );

    assert_eq!(s.ft_balance_of(alice.account_id()).await?.0, 0);

    Ok(())
}
