#![cfg(not(windows))]

use std::collections::HashSet;

use near_sdk::{
    serde::Deserialize,
    serde_json::{self, json},
};
use tokio::join;
use workspaces::{Account, AccountId, Contract};

const WASM: &[u8] = include_bytes!("../../target/wasm32-unknown-unknown/release/rbac.wasm");

#[derive(Deserialize, Clone, Default, Debug, PartialEq, Eq)]
#[serde(crate = "near_sdk::serde")]
struct ContractSchema {
    pub alpha: u32,
    pub beta: u32,
    pub gamma: u32,
    pub delta: u32,
}

struct Setup {
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize, wasm: &[u8]) -> Setup {
    let worker = workspaces::sandbox().await.unwrap();

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
    let Setup { contract, accounts } = setup(4, WASM).await;

    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];
    let daisy = &accounts[3];

    let call = |who: Account, contract: AccountId, method: String, args: Vec<u8>| async move {
        who.call(&contract, &method)
            .args(args)
            .transact()
            .await
            .unwrap()
            .unwrap()
    };

    let acquire_role = |who: Account, role: &str| {
        call(
            who,
            contract.id().clone(),
            "acquire_role".to_string(),
            serde_json::to_vec(&json!({ "role": role })).unwrap(),
        )
    };

    let count_members = |contract: Contract, role: &str| {
        let role = role.to_string();
        async move {
            contract
                .view("count_members")
                .args_json(json!({ "role": role }))
                .await
                .unwrap()
                .json::<u32>()
                .unwrap()
        }
    };

    let members = |contract: Contract, role: &str| {
        let role = role.to_string();
        async move {
            contract
                .view("members")
                .args_json(json!({ "role": role }))
                .await
                .unwrap()
                .json::<HashSet<String>>()
                .unwrap()
        }
    };

    join!(
        // alice has every role
        acquire_role(alice.clone(), "a"),
        acquire_role(alice.clone(), "b"),
        acquire_role(alice.clone(), "g"),
        acquire_role(alice.clone(), "d"),
        // duplicate alice roles should have no effect
        acquire_role(alice.clone(), "a"),
        acquire_role(alice.clone(), "b"),
        acquire_role(alice.clone(), "g"),
        acquire_role(alice.clone(), "d"),
        // bob has same roles as alice
        acquire_role(bob.clone(), "a"),
        acquire_role(bob.clone(), "b"),
        acquire_role(bob.clone(), "g"),
        acquire_role(bob.clone(), "d"),
        // charlie has the first two roles
        acquire_role(charlie.clone(), "a"),
        acquire_role(charlie.clone(), "b"),
        // daisy has no roles
    );

    call(
        alice.clone(),
        contract.id().clone(),
        "requires_alpha".to_string(),
        vec![],
    )
    .await;

    call(
        charlie.clone(),
        contract.id().clone(),
        "requires_alpha".to_string(),
        vec![],
    )
    .await;

    call(
        alice.clone(),
        contract.id().clone(),
        "requires_beta".to_string(),
        vec![],
    )
    .await;

    call(
        bob.clone(),
        contract.id().clone(),
        "requires_gamma".to_string(),
        vec![],
    )
    .await;

    call(
        alice.clone(),
        contract.id().clone(),
        "requires_delta".to_string(),
        vec![],
    )
    .await;

    let schema = contract
        .view("get")
        .await
        .unwrap()
        .json::<ContractSchema>()
        .unwrap();

    assert_eq!(
        schema,
        ContractSchema {
            alpha: 2,
            beta: 1,
            gamma: 1,
            delta: 1,
        },
    );

    let (members_a, members_b, members_g, members_d, count_a, count_b, count_g, count_d) = join!(
        members(contract.clone(), "a"),
        members(contract.clone(), "b"),
        members(contract.clone(), "g"),
        members(contract.clone(), "d"),
        count_members(contract.clone(), "a"),
        count_members(contract.clone(), "b"),
        count_members(contract.clone(), "g"),
        count_members(contract.clone(), "d"),
    );

    let (alice_str, bob_str, charlie_str, _daisy_str) = (
        alice.id().to_string(),
        bob.id().to_string(),
        charlie.id().to_string(),
        daisy.id().to_string(),
    );

    assert_eq!(count_a, 3);
    assert_eq!(count_b, 3);
    assert_eq!(count_g, 2);
    assert_eq!(count_d, 2);

    assert_eq!(
        members_a,
        [alice_str.clone(), bob_str.clone(), charlie_str.clone()].into(),
    );
    assert_eq!(
        members_b,
        [alice_str.clone(), bob_str.clone(), charlie_str].into(),
    );
    assert_eq!(members_g, [alice_str.clone(), bob_str.clone()].into());
    assert_eq!(members_d, [alice_str, bob_str].into());
}

#[tokio::test]
#[should_panic = "Unauthorized role"]
async fn fail_missing_role() {
    let Setup { contract, accounts } = setup(1, WASM).await;

    let alice = &accounts[0];

    alice
        .call(contract.id(), "requires_alpha")
        .transact()
        .await
        .unwrap()
        .unwrap();
}
