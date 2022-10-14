#![cfg(not(windows))]

use near_sdk::serde_json::json;
use workspaces::{Account, Contract};

const WASM: &[u8] =
    include_bytes!("../../target/wasm32-unknown-unknown/release/counter_multisig.wasm");

struct Setup {
    pub contract: Contract,
    pub accounts: Vec<Account>,
}

/// Setup for individual tests
async fn setup(num_accounts: usize) -> Setup {
    let worker = workspaces::sandbox().await.unwrap();

    // Initialize contract
    let contract = worker.dev_deploy(&WASM.to_vec()).await.unwrap();
    contract.call("new").transact().await.unwrap().unwrap();

    // Initialize user accounts
    let mut accounts = vec![];
    for _ in 0..(num_accounts + 1) {
        accounts.push(worker.dev_create_account().await.unwrap());
    }

    Setup { contract, accounts }
}

async fn setup_roles(num_accounts: usize) -> Setup {
    let s = setup(num_accounts).await;

    for account in s.accounts[..s.accounts.len() - 1].iter() {
        account
            .call(s.contract.id(), "obtain_multisig_permission")
            .transact()
            .await
            .unwrap()
            .unwrap();
    }

    s
}

#[tokio::test]
async fn success() {
    let Setup { contract, accounts } = setup_roles(3).await;

    let alice = &accounts[0];
    let bob = &accounts[1];
    let charlie = &accounts[2];

    let create_request = |account: &Account, fname: &str| {
        let fname = fname.to_string();
        let account = account.clone();
        let contract_id = contract.id();
        async move {
            account
                .clone()
                .call(contract_id, &fname)
                .transact()
                .await
                .unwrap()
                .json::<u32>()
                .unwrap()
        }
    };

    // Increment
    let request_id = create_request(alice, "request_increment").await;

    let is_approved = |request_id: u32| {
        let view = contract.view(
            "is_approved",
            json!({ "request_id": request_id })
                .to_string()
                .as_bytes()
                .to_vec(),
        );
        async move { view.await.unwrap().json::<bool>().unwrap() }
    };

    assert!(!is_approved(request_id).await);

    let approve = |account: Account, request_id: u32| {
        let contract_id = contract.id();
        async move {
            account
                .call(contract_id, "approve")
                .args_json(json!({ "request_id": request_id }))
                .transact()
                .await
                .unwrap()
        }
    };

    approve(alice.clone(), request_id).await.unwrap();

    assert!(!is_approved(request_id).await);

    approve(bob.clone(), request_id).await.unwrap();

    assert!(is_approved(request_id).await);

    approve(charlie.clone(), request_id).await.unwrap();

    assert!(is_approved(request_id).await);

    let get_counter = || async {
        contract
            .view("get_counter", vec![])
            .await
            .unwrap()
            .json::<u32>()
            .unwrap()
    };

    let counter = get_counter().await;

    assert_eq!(counter, 0);

    let execute = |account: Account, request_id: u32| {
        let contract_id = contract.id();
        async move {
            account
                .call(contract_id, "execute")
                .args_json(json!({ "request_id": request_id }))
                .transact()
                .await
                .unwrap()
                .json::<u32>()
                .unwrap()
        }
    };

    let result = execute(alice.clone(), request_id).await;

    assert_eq!(result, 1);

    let counter = get_counter().await;

    assert_eq!(counter, 1);

    let request_id = create_request(bob, "request_increment").await;
    approve(bob.clone(), request_id).await.unwrap();
    approve(alice.clone(), request_id).await.unwrap();
    let result = execute(bob.clone(), request_id).await;
    let counter = get_counter().await;
    assert_eq!(result, counter);
    assert_eq!(counter, 2);

    let request_id = create_request(charlie, "request_decrement").await;
    approve(bob.clone(), request_id).await.unwrap();
    approve(charlie.clone(), request_id).await.unwrap();
    let result = execute(alice.clone(), request_id).await;
    let counter = get_counter().await;
    assert_eq!(result, counter);
    assert_eq!(counter, 1);

    let request_id = create_request(charlie, "request_reset").await;
    approve(bob.clone(), request_id).await.unwrap();
    approve(alice.clone(), request_id).await.unwrap();
    let result = execute(alice.clone(), request_id).await;
    let counter = get_counter().await;
    assert_eq!(result, counter);
    assert_eq!(counter, 0);
}
