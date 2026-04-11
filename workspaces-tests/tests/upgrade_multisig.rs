use near_api::Contract;
use near_sdk::{json_types::Base64VecU8, near, serde_json::json};
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
        .make_contract("upgrade_old_multisig", "upgrade_old_multisig", json!({}))
        .await?;

    Ok(Setup { contract, handle })
}

#[derive(Debug, Clone)]
#[near(serializers = [json])]
pub enum ContractAction {
    Upgrade { code: Base64VecU8 },
}

impl Setup {
    transaction! { fn execute(request_id: u32) }
    transaction! { fn approve(request_id: u32) }
    transaction! { fn request(request: ContractAction) -> u32 }

    read_only! { fn get_bar() -> u64 }
}

#[tokio::test]
async fn upgrade_multisig() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.contract.as_account();

    let code = Base64VecU8::from(s.handle.load_wasm("upgrade_new").await?);

    let request_id = s
        .request(&alice, None, ContractAction::Upgrade { code })
        .await?;

    s.approve(&alice, None, request_id).await?;

    s.execute(&alice, None, request_id).await?;

    let new_val = s.get_bar().await?;

    assert_eq!(new_val, 0);

    Ok(())
}
