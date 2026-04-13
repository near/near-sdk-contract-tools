use near_api::Contract;
use near_sdk::{NearToken, serde_json::json};
use testresult::TestResult;
use workspaces_tests::{Handle, read_only, transaction};

struct Setup {
    pub handle: Handle,
    pub contract: Contract,
}

async fn setup() -> TestResult<Setup> {
    let handle = Handle::new().await;
    let contract = handle
        .make_contract("storage_fee", "storage_fee", json!({}))
        .await?;

    Ok(Setup { contract, handle })
}

impl Setup {
    read_only! { fn storage_byte_cost() -> NearToken }
    transaction! { fn store(item: String) }
}

#[tokio::test]
async fn storage_fee() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.handle.make_account("alice").await?;

    let fetch_balance = || async {
        alice
            .view()
            .fetch_from(&s.handle.network)
            .await
            .unwrap()
            .data
            .amount
    };

    let balance_start = fetch_balance().await;

    let byte_cost = s.storage_byte_cost().await?;

    let num_bytes = NearToken::from_near(1)
        .as_yoctonear()
        .saturating_div(byte_cost.as_yoctonear());
    let payload = "0".repeat(usize::try_from(num_bytes).unwrap());
    // This is the absolute minimum this payload should require to store (uncompressed)
    let minimum_storage_fee = byte_cost.saturating_mul(num_bytes);
    let gas_price = near_api::Chain::block()
        .fetch_from(&s.handle.network)
        .await?
        .header
        .gas_price;

    let go = || async {
        let balance_before = fetch_balance().await;

        // Should receive back about 9 NEAR as refund
        let r = s
            .store(&alice, Some(NearToken::from_near(10)), &payload)
            .await
            .unwrap();

        let balance_after = fetch_balance().await;

        // How much was actually charged to the account?
        // Note that there will be *some* overhead, e.g. collection indexing
        let net_fee = balance_before
            .saturating_sub(balance_after)
            .saturating_sub(gas_price.saturating_mul(r.total_gas_burnt.as_gas() as u128));

        assert!(net_fee >= minimum_storage_fee);

        // Sanity/validity check / allow up to 100 bytes worth of additional storage to be charged
        assert!(net_fee.saturating_sub(minimum_storage_fee) < byte_cost.saturating_mul(100));
    };

    for _ in 0..5 {
        go().await;
    }

    let balance_end = fetch_balance().await;
    assert!(balance_start.saturating_sub(balance_end) >= minimum_storage_fee.saturating_mul(5));

    Ok(())
}
