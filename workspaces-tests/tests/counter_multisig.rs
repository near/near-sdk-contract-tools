use near_api::{Account, Contract};
use near_sdk::serde_json::json;
use pretty_assertions::assert_eq;
use testresult::TestResult;
use workspaces_tests::{Handle, read_only, transaction};

struct Setup {
    pub handle: Handle,
    pub contract: Contract,
}

async fn setup() -> TestResult<Setup> {
    let handle = Handle::new().await;
    let contract = handle
        .make_contract("counter_multisig", "counter_multisig", json!({}))
        .await?;
    Ok(Setup { handle, contract })
}

impl Setup {
    async fn setup_role(&self, account_id: impl Into<String>) -> TestResult<Account> {
        let account = self.handle.make_account(account_id).await?;
        self.contract
            .call_function("obtain_multisig_permission", json!({}))
            .transaction()
            .with_signer(account.account_id().clone(), self.handle.default_signer())
            .send_to(&self.handle.network)
            .await?
            .assert_success();
        Ok(account)
    }

    read_only! { fn is_approved(request_id: u32) -> bool }
    read_only! { fn get_counter() -> u32 }
    transaction! { fn request_increment() -> u32 }
    transaction! { fn request_decrement() -> u32 }
    transaction! { fn request_reset() -> u32 }
    transaction! { fn approve(request_id: u32) }
    transaction! { fn execute(request_id: u32) -> u32 }
}

#[tokio::test]
async fn success() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_role("alice").await?;
    let bob = s.setup_role("bob").await?;
    let charlie = s.setup_role("charlie").await?;

    // Increment
    let request_id = s.request_increment(&alice, None).await?;

    assert!(!s.is_approved(request_id).await?);

    s.approve(&alice, None, request_id).await?;

    assert!(!s.is_approved(request_id).await?);

    s.approve(&bob, None, request_id).await?;

    assert!(s.is_approved(request_id).await?);

    s.approve(&charlie, None, request_id).await?;

    assert!(s.is_approved(request_id).await?);

    let counter = s.get_counter().await?;

    assert_eq!(counter, 0);

    let result = s.execute(&alice, None, request_id).await?;

    assert_eq!(result, 1);

    let counter = s.get_counter().await?;

    assert_eq!(counter, 1);

    let request_id = s.request_increment(&bob, None).await?;
    s.approve(&bob, None, request_id).await?;
    s.approve(&alice, None, request_id).await?;
    let result = s.execute(&bob, None, request_id).await?;
    let counter = s.get_counter().await?;
    assert_eq!(result, counter);
    assert_eq!(counter, 2);

    let request_id = s.request_decrement(&charlie, None).await?;
    s.approve(&bob, None, request_id).await?;
    s.approve(&charlie, None, request_id).await?;
    let result = s.execute(&alice, None, request_id).await?;
    let counter = s.get_counter().await?;
    assert_eq!(result, counter);
    assert_eq!(counter, 1);

    let request_id = s.request_reset(&charlie, None).await?;
    s.approve(&bob, None, request_id).await?;
    s.approve(&alice, None, request_id).await?;
    let result = s.execute(&alice, None, request_id).await?;
    let counter = s.get_counter().await?;
    assert_eq!(result, counter);
    assert_eq!(counter, 0);

    Ok(())
}
