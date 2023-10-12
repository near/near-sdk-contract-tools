#![cfg(not(windows))]

use near_sdk::{
    serde::{Deserialize, Serialize},
    serde_json::{self, json},
};
use near_workspaces::{Account, AccountId, Contract};
use tokio::join;

const WASM: &[u8] = include_bytes!("../../target/wasm32-unknown-unknown/release/escrow.wasm");

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
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize, wasm: &[u8]) -> Setup {
    let worker = near_workspaces::sandbox().await.unwrap();

    // Initialize user accounts
    let mut accounts = vec![];
    for _ in 0..(num_accounts + 1) {
        accounts.push(worker.dev_create_account().await.unwrap());
    }

    let alice = &accounts[0].clone();

    let contract = alice.deploy(wasm).await.unwrap().unwrap();
    contract.call("new").transact().await.unwrap().unwrap();

    Setup { contract, accounts }
}

#[tokio::test]
async fn happy() {
    let Setup { contract, accounts } = setup(2, WASM).await;

    let alice = &accounts[0];
    let bob = &accounts[1];

    let call = |who: Account, contract: AccountId, method: String, args: Vec<u8>| async move {
        who.call(&contract, &method)
            .args(args)
            .transact()
            .await
            .unwrap()
            .unwrap()
    };

    let assign = |who: Account, colour: PrimaryColour| {
        call(
            who,
            contract.id().clone(),
            "assign".to_string(),
            serde_json::to_vec(&json!({ "colour": colour })).unwrap(),
        )
    };
    let mix = |who: Account, contract: AccountId, colour: PrimaryColour, with: PrimaryColour| async move {
        who.call(&contract, "mix")
            .args(serde_json::to_vec(&json!({ "colour": colour, "with": with })).unwrap())
            .transact()
            .await
            .unwrap()
            .json::<(AccountId, AccountId, SecondaryColour)>()
            .unwrap()
    };
    let alice_colour = PrimaryColour::Red;
    join!(
        assign(alice.clone(), alice_colour.clone()),
        assign(bob.clone(), PrimaryColour::Blue),
    );
    let (pair_x, pair_y, mixed_colour) = mix(
        bob.clone(),
        contract.id().clone(),
        PrimaryColour::Blue,
        alice_colour.clone(),
    )
    .await;

    let locked = contract
        .view("get_locked")
        .args(serde_json::to_vec(&json!({ "colour": alice_colour })).unwrap())
        .await
        .unwrap()
        .json::<bool>()
        .unwrap();

    assert!(!locked);
    assert_eq!(pair_x, bob.clone().id().to_owned());
    assert_eq!(pair_y, alice.clone().id().to_owned());
    assert_eq!(mixed_colour, SecondaryColour::Purple);
}

#[tokio::test]
#[should_panic(expected = "Already locked")]
async fn unhappy_cant_lock() {
    let Setup { contract, accounts } = setup(1, WASM).await;

    let alice = &accounts[0];

    let call = |who: Account, contract: AccountId, method: String, args: Vec<u8>| async move {
        who.call(&contract, &method)
            .args(args)
            .transact()
            .await
            .unwrap()
            .unwrap()
    };

    let assign = |who: Account, colour: PrimaryColour| {
        call(
            who,
            contract.id().clone(),
            "assign".to_string(),
            serde_json::to_vec(&json!({ "colour": colour })).unwrap(),
        )
    };

    join!(
        assign(alice.clone(), PrimaryColour::Red),
        assign(alice.clone(), PrimaryColour::Red),
    );
}
