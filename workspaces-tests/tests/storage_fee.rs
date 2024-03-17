workspaces_tests::near_sdk!();

use near_sdk::{json_types::U128, serde_json::json};
use near_sdk_contract_tools::{compat_near_to_u128, COMPAT_ONE_NEAR};
use near_workspaces::{sandbox, types::NearToken, Account, Contract, DevNetwork, Worker};
use workspaces_tests_utils::ONE_NEAR;

const WASM: &[u8] = include_bytes!("../../target/wasm32-unknown-unknown/release/storage_fee.wasm");

struct Setup<T: DevNetwork> {
    pub worker: Worker<T>,
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

async fn setup<T: DevNetwork>(worker: Worker<T>, num_accounts: usize) -> Setup<T> {
    // Initialize contract
    let contract = worker.dev_deploy(WASM).await.unwrap();
    contract.call("new").transact().await.unwrap().unwrap();

    // Initialize user accounts
    let mut accounts = vec![];
    for _ in 0..(num_accounts + 1) {
        accounts.push(worker.dev_create_account().await.unwrap());
    }

    Setup {
        worker,
        contract,
        accounts,
    }
}

#[tokio::test]
async fn storage_fee() {
    let Setup {
        contract,
        accounts,
        worker,
    } = setup(sandbox().await.unwrap(), 1).await;

    let alice = &accounts[0];
    let balance_start = alice.view_account().await.unwrap().balance;

    let byte_cost = contract
        .view("storage_byte_cost")
        .await
        .unwrap()
        .json::<U128>()
        .unwrap()
        .0;

    let num_bytes: usize = compat_near_to_u128!(COMPAT_ONE_NEAR.saturating_div(byte_cost))
        .try_into()
        .unwrap();
    let payload = "0".repeat(num_bytes);
    // This is the absolute minimum this payload should require to store (uncompressed)
    let minimum_storage_fee = NearToken::from_yoctonear(num_bytes as u128 * byte_cost);
    let gas_price = worker.gas_price().await.unwrap();

    let go = || async {
        let balance_before = alice.view_account().await.unwrap().balance;

        let r = alice
            .call(contract.id(), "store")
            .args_json(json!({
                "item": payload,
            }))
            .deposit(ONE_NEAR.saturating_mul(10)) // Should receive back about 9 NEAR as refund
            .transact()
            .await
            .unwrap()
            .unwrap();

        let balance_after = alice.view_account().await.unwrap().balance;

        // How much was actually charged to the account?
        // Note that there will be *some* overhead, e.g. collection indexing
        let net_fee = balance_before
            .saturating_sub(balance_after)
            .saturating_sub(gas_price.saturating_mul(r.total_gas_burnt.as_gas() as u128));

        assert!(net_fee >= minimum_storage_fee);
        assert!(
            net_fee.saturating_sub(minimum_storage_fee)
                < NearToken::from_yoctonear(byte_cost * 100)
        ); // Sanity/validity check / allow up to 100 bytes worth of additional storage to be charged
    };

    for _ in 0..5 {
        go().await;
    }

    let balance_end = alice.view_account().await.unwrap().balance;
    assert!(balance_start.saturating_sub(balance_end) >= minimum_storage_fee.saturating_mul(5));
}
