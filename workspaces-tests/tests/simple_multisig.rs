use near_api::{Account, Contract};
use near_sdk::serde_json::json;
use pretty_assertions::assert_eq;
use testresult::TestResult;
use workspaces_tests::{Handle, read_only, transaction};

struct Setup {
    pub handle: Handle,
    pub contract: Contract,
}

/// Setup for individual tests
async fn setup() -> TestResult<Setup> {
    let handle = Handle::new().await;
    let contract = handle
        .make_contract("simple_multisig", "simple_multisig", json!({}))
        .await?;

    Ok(Setup { contract, handle })
}

impl Setup {
    async fn setup_account(&self, account: impl Into<String>) -> TestResult<Account> {
        let account = self.handle.make_account(account).await?;
        self.obtain_multisig_permission(&account, None).await?;
        Ok(account)
    }

    transaction! { fn obtain_multisig_permission() }
    transaction! { fn request(action: String) -> u32 }
    transaction! { fn approve(request_id: u32) }
    read_only! { fn is_approved(request_id: u32) -> bool }
    transaction! { fn execute(request_id: u32) -> String }
}

#[tokio::test]
async fn successful_request() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice").await?;
    let bob = s.setup_account("bob").await?;
    let charlie = s.setup_account("charlie").await?;

    let request_id = s.request(&alice, None, "hello").await?;

    assert!(!s.is_approved(request_id).await?);

    s.approve(&alice, None, request_id).await?;

    assert!(!s.is_approved(request_id).await?);

    s.approve(&bob, None, request_id).await?;

    assert!(s.is_approved(request_id).await?);

    s.approve(&charlie, None, request_id).await?;

    assert!(s.is_approved(request_id).await?);

    let exec_result = s.execute(&charlie, None, request_id).await?;

    assert_eq!(exec_result, "hello");

    Ok(())
}

#[tokio::test]
#[should_panic = "UnauthorizedAccount"]
async fn unauthorized_account() {
    let s = setup().await.unwrap();

    let alice = s.setup_account("alice").await.unwrap();
    let unauthorized_account = s.handle.make_account("unauthorized_account").await.unwrap();

    let request_id = s.request(&alice, None, "hello").await.unwrap();

    s.approve(&unauthorized_account, None, request_id)
        .await
        .unwrap();
}
