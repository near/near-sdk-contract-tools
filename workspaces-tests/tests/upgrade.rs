use near_api::{
    Account, Contract,
    types::transaction::result::{ExecutionResult, Value},
};
use near_sdk::{
    AccountId,
    borsh::{self, BorshSerialize},
    serde::Serialize,
    serde_json::{self, json},
};
use pretty_assertions::assert_eq;
use testresult::TestResult;
use workspaces_tests::{Handle, Y, read_only, transaction};

#[derive(BorshSerialize)]
#[borsh(crate = "near_sdk::borsh")]
struct ArgsBorsh {
    pub code: Vec<u8>,
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
struct ArgsJson {
    pub code: near_sdk::json_types::Base64VecU8,
}

struct Setup {
    pub handle: Handle,
    pub contract: Contract,
}

/// Setup for individual tests
async fn setup(wasm: &str) -> TestResult<Setup> {
    let handle = Handle::new().await;
    let contract = handle.make_contract(wasm, wasm, json!({})).await?;

    Ok(Setup { contract, handle })
}

impl Setup {
    async fn new_wasm(&self) -> TestResult<Vec<u8>> {
        self.handle.load_wasm("upgrade_new").await
    }

    async fn replace_owner(&self, account: &Account) -> TestResult<()> {
        self.own_propose_owner(&self.contract.as_account(), Y, account.account_id().clone())
            .await?;
        self.own_accept_owner(account, Y).await?;
        Ok(())
    }

    async fn upgrade(
        &self,
        account: &Account,
        args_raw: Vec<u8>,
    ) -> TestResult<ExecutionResult<Value>> {
        Ok(self
            .contract
            .call_function_raw("upgrade", args_raw)
            .transaction()
            .max_gas()
            .with_signer(account.account_id().clone(), self.handle.default_signer())
            .send_to(&self.handle.network)
            .await?
            .assert_success())
    }

    transaction! { fn increment_foo() }
    read_only! { fn get_foo() -> u32 }
    read_only! { fn get_bar() -> u32 }

    read_only! { fn own_get_owner() -> Option<AccountId> }
    read_only! { fn own_get_proposed_owner() -> Option<AccountId> }
    transaction! { fn own_renounce_owner() }
    transaction! { fn own_propose_owner(account_id: Option<AccountId>) }
    transaction! { fn own_accept_owner() }

    async fn perform_upgrade_test(&self, args: Vec<u8>) -> TestResult<()> {
        let alice = self.handle.make_account("alice").await?;
        self.replace_owner(&alice).await?;

        self.increment_foo(&alice, None).await?;

        let val = self.get_foo().await?;

        assert_eq!(val, 1);

        self.upgrade(&alice, args).await?;

        let new_val = self.get_bar().await?;

        assert_eq!(new_val, 1);

        Ok(())
    }

    async fn fail_owner(&self, args: Vec<u8>) -> TestResult<()> {
        let alice = self.handle.make_account("alice").await?;
        self.replace_owner(&alice).await?;
        let bob = self.handle.make_account("bob").await?;

        self.upgrade(&bob, args).await?;
        Ok(())
    }
}

#[tokio::test]
async fn upgrade_borsh() -> TestResult<()> {
    let s = setup("upgrade_old_borsh").await?;
    s.perform_upgrade_test(
        borsh::to_vec(&ArgsBorsh {
            code: s.new_wasm().await?,
        })
        .unwrap(),
    )
    .await?;
    Ok(())
}

#[tokio::test]
// #[ignore]
async fn upgrade_jsonbase64() -> TestResult<()> {
    let s = setup("upgrade_old_jsonbase64").await?;
    // For some reason this test fails only on GitHub Actions due to a running-out-of-gas error.
    // if std::env::var_os("GITHUB_ACTIONS").is_some() {
    //     eprintln!("Skipping upgrade_jsonbase64 test on GitHub Actions.");
    //     return;
    // }
    s.perform_upgrade_test(
        serde_json::to_vec(&ArgsJson {
            code: s.new_wasm().await?.into(),
        })
        .unwrap(),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn upgrade_raw() -> TestResult<()> {
    let s = setup("upgrade_old_raw").await?;
    s.perform_upgrade_test(s.new_wasm().await?).await?;
    Ok(())
}

#[tokio::test]
#[should_panic = "Failed to deserialize input from Borsh."]
async fn upgrade_failure_blank_wasm() {
    let s = setup("upgrade_old_borsh").await.unwrap();
    s.perform_upgrade_test(vec![]).await.unwrap();
}

#[tokio::test]
#[should_panic = "MethodResolveError(MethodNotFound)"]
async fn upgrade_failure_no_upgrade() {
    let s = setup("upgrade_bad").await.unwrap();

    let alice = s.handle.make_account("alice").await.unwrap();
    s.replace_owner(&alice).await.unwrap();

    s.upgrade(&alice, vec![]).await.unwrap();
}

#[tokio::test]
#[should_panic = "MethodResolveError(MethodNotFound)"]
async fn upgrade_failure_random_wasm() {
    let s = setup("counter_multisig").await.unwrap();

    let alice = s.handle.make_account("alice").await.unwrap();
    s.replace_owner(&alice).await.unwrap();

    s.upgrade(&alice, vec![]).await.unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Owner only"]
async fn upgrade_failure_not_owner_borsh() {
    let s = setup("upgrade_old_borsh").await.unwrap();
    s.fail_owner(
        borsh::to_vec(&ArgsBorsh {
            code: s.new_wasm().await.unwrap(),
        })
        .unwrap(),
    )
    .await
    .unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Owner only"]
async fn upgrade_failure_not_owner_jsonbase64() {
    let s = setup("upgrade_old_jsonbase64").await.unwrap();
    s.fail_owner(
        serde_json::to_vec(&ArgsJson {
            code: s.new_wasm().await.unwrap().into(),
        })
        .unwrap(),
    )
    .await
    .unwrap();
}

#[tokio::test]
#[should_panic = "Smart contract panicked: Owner only"]
async fn upgrade_failure_not_owner_raw() {
    let s = setup("upgrade_old_raw").await.unwrap();
    s.fail_owner(s.new_wasm().await.unwrap()).await.unwrap();
}
