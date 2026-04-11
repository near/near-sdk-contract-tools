use near_api::Contract;
use near_sdk::{
    AccountId,
    serde::{Deserialize, Serialize},
    serde_json::json,
};
use pretty_assertions::assert_eq;
use testresult::TestResult;
use tokio::join;
use workspaces_tests::{Handle, read_only, transaction};

#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
struct ContractSchema {}

#[derive(Serialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum PrimaryColour {
    Red,
    Yellow,
    Blue,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
pub enum SecondaryColour {
    Orange,
    Green,
    Purple,
}

struct Setup {
    pub handle: Handle,
    pub contract: Contract,
}

/// Setup for individual tests
async fn setup() -> TestResult<Setup> {
    let handle = Handle::new().await;
    let contract = handle.make_contract("escrow", "escrow", json!({})).await?;

    Ok(Setup { contract, handle })
}

impl Setup {
    transaction! { fn assign(colour: PrimaryColour) }
    transaction! { fn mix(colour: PrimaryColour, with: PrimaryColour) -> (AccountId, AccountId, SecondaryColour) }
    read_only! { fn get_locked(colour: PrimaryColour) -> bool }
}

#[tokio::test]
async fn happy() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.handle.make_account("alice").await?;
    let bob = s.handle.make_account("bob").await?;

    let alice_colour = PrimaryColour::Red;
    join!(
        async { s.assign(&alice, None, alice_colour.clone()).await.unwrap() },
        async { s.assign(&bob, None, PrimaryColour::Blue).await.unwrap() },
    );
    let (pair_x, pair_y, mixed_colour) = s
        .mix(&bob, None, PrimaryColour::Blue, alice_colour.clone())
        .await?;

    let locked = s.get_locked(alice_colour).await?;

    assert!(!locked);
    assert_eq!(pair_x, bob.account_id().to_owned());
    assert_eq!(pair_y, alice.account_id().to_owned());
    assert_eq!(mixed_colour, SecondaryColour::Purple);

    Ok(())
}

#[tokio::test]
#[should_panic(expected = "Already locked")]
async fn unhappy_cant_lock() {
    let s = setup().await.unwrap();
    let alice = s.handle.make_account("alice").await.unwrap();

    s.assign(&alice, None, PrimaryColour::Red).await.unwrap();
    s.assign(&alice, None, PrimaryColour::Red).await.unwrap();
}
