use std::collections::HashSet;

use near_api::Contract;
use near_sdk::{AccountId, serde::Deserialize, serde_json::json};
use pretty_assertions::assert_eq;
use testresult::TestResult;
use workspaces_tests::{Handle, read_only, transaction};

#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
struct ContractSchema {
    pub alpha: u32,
    pub beta: u32,
    pub gamma: u32,
    pub delta: u32,
}

struct Setup {
    pub handle: Handle,
    pub contract: Contract,
}

impl Setup {
    transaction! { fn acquire_role(role: String) }
    read_only! { fn members(role: String) -> Vec<AccountId> }
    read_only! { fn count_members(role: String) -> u32 }
    transaction! { fn requires_alpha() }
    transaction! { fn requires_beta() }
    transaction! { fn requires_gamma() }
    transaction! { fn requires_delta() }
    read_only! { fn get() -> ContractSchema }
}

/// Setup for individual tests
async fn setup() -> TestResult<Setup> {
    let handle = Handle::new().await;
    let contract = handle.make_contract("rbac", "rbac", json!({})).await?;

    Ok(Setup { contract, handle })
}

#[tokio::test]
async fn happy() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.handle.make_account("alice").await?;
    let bob = s.handle.make_account("bob").await?;
    let charlie = s.handle.make_account("charlie").await?;
    let _daisy = s.handle.make_account("daisy").await?;

    // alice has every role
    s.acquire_role(&alice, None, "a").await?;
    s.acquire_role(&alice, None, "b").await?;
    s.acquire_role(&alice, None, "g").await?;
    s.acquire_role(&alice, None, "d").await?;
    // duplicate alice roles should have no effect
    s.acquire_role(&alice, None, "a").await?;
    s.acquire_role(&alice, None, "b").await?;
    s.acquire_role(&alice, None, "g").await?;
    s.acquire_role(&alice, None, "d").await?;
    // bob has same roles as alice
    s.acquire_role(&bob, None, "a").await?;
    s.acquire_role(&bob, None, "b").await?;
    s.acquire_role(&bob, None, "g").await?;
    s.acquire_role(&bob, None, "d").await?;
    // charlie has the first two roles
    s.acquire_role(&charlie, None, "a").await?;
    s.acquire_role(&charlie, None, "b").await?;
    // daisy has no roles

    s.requires_alpha(&alice, None).await?;
    s.requires_alpha(&charlie, None).await?;

    s.requires_beta(&alice, None).await?;

    s.requires_gamma(&bob, None).await?;

    s.requires_delta(&alice, None).await?;

    let schema = s.get().await?;

    assert_eq!(
        schema,
        ContractSchema {
            alpha: 2,
            beta: 1,
            gamma: 1,
            delta: 1,
        },
    );

    let members_a = s.members("a").await?;
    let members_b = s.members("b").await?;
    let members_g = s.members("g").await?;
    let members_d = s.members("d").await?;
    let count_a = s.count_members("a").await?;
    let count_b = s.count_members("b").await?;
    let count_g = s.count_members("g").await?;
    let count_d = s.count_members("d").await?;

    assert_eq!(count_a, 3);
    assert_eq!(count_b, 3);
    assert_eq!(count_g, 2);
    assert_eq!(count_d, 2);

    let abc = HashSet::<AccountId>::from_iter([
        alice.account_id().clone(),
        bob.account_id().clone(),
        charlie.account_id().clone(),
    ]);
    let ab =
        HashSet::<AccountId>::from_iter([alice.account_id().clone(), bob.account_id().clone()]);

    assert_eq!(HashSet::from_iter(members_a), abc);
    assert_eq!(HashSet::from_iter(members_b), abc);
    assert_eq!(HashSet::from_iter(members_g), ab);
    assert_eq!(HashSet::from_iter(members_d), ab);

    Ok(())
}

#[tokio::test]
#[should_panic = "Unauthorized role"]
async fn fail_missing_role() {
    let s = setup().await.unwrap();

    let alice = s.handle.make_account("alice").await.unwrap();

    s.requires_alpha(&alice, None).await.unwrap();
}
