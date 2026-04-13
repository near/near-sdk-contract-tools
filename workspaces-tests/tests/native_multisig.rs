use std::time::Duration;

use near_api::{
    Account, Contract,
    types::{
        AccessKeyPermission,
        transaction::result::{ExecutionResult, Value},
    },
};
use near_crypto::{KeyType, SecretKey};
use near_sdk::{AccountId, Gas, NearToken, serde_json::json};
use near_sdk_contract_tools::approval::native_transaction_action::PromiseAction;
use pretty_assertions::assert_eq;
use testresult::TestResult;
use tokio::time::sleep;
use workspaces_tests::{Handle, read_only, transaction};

struct Setup {
    pub handle: Handle,
    pub contract: Contract,
}

async fn setup() -> TestResult<Setup> {
    // Initialize contract
    let handle = Handle::new().await;
    let contract = handle
        .make_contract("native_multisig", "native_multisig", json!({}))
        .await?;

    Ok(Setup { handle, contract })
}

impl Setup {
    async fn setup_account(&self, account: impl Into<String>) -> TestResult<Account> {
        let account = self.handle.make_account(account.into()).await?;

        self.obtain_multisig_permission(&account, None).await?;

        Ok(account)
    }

    async fn execute_actions(
        &self,
        signer_1: &Account,
        signer_2: &Account,
        actions: Vec<PromiseAction>,
    ) -> TestResult<()> {
        let request_id = self
            .request(signer_1, None, self.contract.account_id(), actions)
            .await?;

        self.double_approve_and_execute(signer_1, signer_2, signer_1, request_id)
            .await?;

        // Finality is apparently insufficient here, as I was still getting some
        // errors on both Testnet and Sandbox if I didn't add the delay.
        sleep(Duration::from_secs(1)).await;

        Ok(())
    }

    async fn double_approve_and_execute(
        &self,
        signer_1: &Account,
        signer_2: &Account,
        executor: &Account,
        request_id: u32,
    ) -> TestResult<ExecutionResult<Value>> {
        self.approve(signer_1, None, request_id).await?;
        self.approve(signer_2, None, request_id).await?;
        self.execute(executor, None, request_id).await
    }

    transaction! { fn obtain_multisig_permission() }
    transaction! { fn request(receiver_id: AccountId, actions: Vec<PromiseAction>) -> u32 }
    transaction! { fn approve(request_id: u32) }
    transaction! { fn execute(request_id: u32) }
    read_only! { fn is_approved(request_id: u32) -> bool }
}

#[tokio::test]
async fn stake() -> TestResult<()> {
    let s = setup().await?;

    const MINIMUM_STAKE: NearToken = NearToken::from_yoctonear(800_000_000_000_000_000_000_000_000);

    // for some reason patch_state is returning HTTP error 400
    {
        let tokens_please_id: AccountId = "tokens_please".parse()?;
        s.handle
            .sandbox
            .create_account(tokens_please_id.clone())
            .initial_balance(MINIMUM_STAKE.saturating_mul(4))
            .send()
            .await?;
        Account(tokens_please_id)
            .delete_account_with_beneficiary(s.contract.account_id().clone())
            .with_signer(s.handle.default_signer())
            .send_to(&s.handle.network)
            .await?
            .assert_success();
    }

    let alice = s.setup_account("alice").await?;
    let bob = s.setup_account("bob").await?;

    let secret_key = SecretKey::from_random(KeyType::ED25519);
    let public_key = secret_key.public_key();

    let contract_before = s
        .contract
        .as_account()
        .view()
        .fetch_from(&s.handle.network)
        .await?
        .data;
    assert_eq!(
        contract_before.locked.as_yoctonear(),
        0,
        "Account should start with no staked tokens"
    );

    let amount = MINIMUM_STAKE.saturating_mul(2);

    let request_id = s
        .request(
            &alice,
            None,
            s.contract.account_id(),
            [PromiseAction::Stake {
                amount,
                public_key: public_key.to_string(),
            }],
        )
        .await?;

    s.double_approve_and_execute(&alice, &bob, &alice, request_id)
        .await?;

    let contract_after = s
        .contract
        .as_account()
        .view()
        .fetch_from(&s.handle.network)
        .await?
        .data;

    assert_eq!(
        contract_after.locked, amount,
        "Locked amount should be equal to the amount staked"
    );

    Ok(())
}

#[tokio::test]
async fn delete_account() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice").await?;
    let bob = s.setup_account("bob").await?;

    let alice_balance_before = alice
        .tokens()
        .near_balance()
        .fetch_from(&s.handle.network)
        .await?
        .total;
    let contract_balance_before = s
        .contract
        .as_account()
        .tokens()
        .near_balance()
        .fetch_from(&s.handle.network)
        .await?
        .total;

    let request_id = s
        .request(
            &alice,
            None,
            s.contract.account_id(),
            [PromiseAction::DeleteAccount {
                beneficiary_id: alice.account_id().clone(),
            }],
        )
        .await?;

    s.double_approve_and_execute(&alice, &bob, &alice, request_id)
        .await?;

    s.contract
        .as_account()
        .view()
        .fetch_from(&s.handle.network)
        .await
        .expect_err("Contract account should be deleted");

    let alice_balance_after = alice
        .tokens()
        .near_balance()
        .fetch_from(&s.handle.network)
        .await?
        .total;

    let gas_price = near_api::Chain::block()
        .fetch_from(&s.handle.network)
        .await?
        .header
        .gas_price;

    const MAX_GAS: u128 = 300_000_000_000_000;

    assert!(
        alice_balance_after.as_yoctonear().abs_diff(
            alice_balance_before
                .saturating_add(contract_balance_before)
                .as_yoctonear()
        ) <= gas_price.saturating_mul(MAX_GAS).as_yoctonear(),
        "All contract account funds (sans gas) transfer to the beneficiary account",
    );

    Ok(())
}

#[tokio::test]
async fn create_account_transfer_deploy_contract_function_call() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice").await?;
    let bob = s.setup_account("bob").await?;

    let new_account = near_api::Account(s.contract.account_id().sub_account("new")?);

    new_account
        .view()
        .fetch_from(&s.handle.network)
        .await
        .expect_err("New account does not exist yet");

    let request_id = s
        .request(
            &alice,
            None,
            new_account.account_id(),
            [
                PromiseAction::CreateAccount,
                PromiseAction::Transfer {
                    amount: NearToken::from_near(30),
                },
                PromiseAction::DeployContract {
                    code: s.handle.load_wasm("basic_adder").await?.into(),
                },
                PromiseAction::FunctionCall {
                    function_name: "new".into(),
                    arguments: vec![].into(),
                    amount: NearToken::from_yoctonear(0),
                    gas: Gas::from_tgas(1),
                },
            ],
        )
        .await?;

    s.double_approve_and_execute(&alice, &bob, &alice, request_id)
        .await?;

    let state = new_account.view().fetch_from(&s.handle.network).await?.data;
    assert!(state.amount >= NearToken::from_near(30));

    let result = new_account
        .as_contract()
        .call_function("add_five", json!({ "value": 5 }))
        .read_only::<u32>()
        .fetch_from(&s.handle.network)
        .await?
        .data;

    assert_eq!(
        result, 10,
        "Contract is deployed to child account and is working"
    );

    Ok(())
}

#[tokio::test]
async fn add_both_access_key_kinds_and_delete() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice").await?;
    let bob = s.setup_account("bob").await?;

    // Add full-access key
    let full_access_key = {
        let secret_key = SecretKey::from_random(KeyType::ED25519);
        let new_public_key = secret_key.public_key();
        let new_public_key_string = new_public_key.to_string();

        let keys_before = s
            .contract
            .as_account()
            .list_keys()
            .fetch_from(&s.handle.network)
            .await?
            .data;

        // workspaces::types::PublicKey wrapper type's contents are package-private
        // and there is no Display/.to_string() implementation.
        let new_key_json_string = near_sdk::serde_json::to_string(&new_public_key_string).unwrap();

        assert!(
            !keys_before
                .iter()
                .any(
                    |(public_key, _access_key)| near_sdk::serde_json::to_string(&public_key)
                        .unwrap()
                        == new_key_json_string
                ),
            "New key does not exist in access keys before being added"
        );

        s.execute_actions(
            &alice,
            &bob,
            vec![PromiseAction::AddFullAccessKey {
                public_key: new_public_key_string.clone(),
                nonce: None,
            }],
        )
        .await?;

        let keys_after = s
            .contract
            .as_account()
            .list_keys()
            .fetch_from(&s.handle.network)
            .await?
            .data;

        assert_eq!(
            keys_before.len() + 1,
            keys_after.len(),
            "There should be exactly one additional access key"
        );

        let key = keys_after
            .iter()
            .find(|(public_key, _access_key)| {
                near_sdk::serde_json::to_string(&public_key).unwrap() == new_key_json_string
            })
            .unwrap();

        match &key.1.permission {
            AccessKeyPermission::FullAccess => {}
            _ => panic!("Expected full access key"),
        }

        new_public_key_string
    };

    // Add function-call access key
    let function_call_key = {
        let secret_key = SecretKey::from_random(KeyType::ED25519);
        let new_public_key = secret_key.public_key();
        let new_public_key_string = new_public_key.to_string();

        let keys_before = s
            .contract
            .as_account()
            .list_keys()
            .fetch_from(&s.handle.network)
            .await?
            .data;

        // workspaces::types::PublicKey wrapper type's contents are package-private
        // and there is no Display/.to_string() implementation.
        let new_key_json_string = near_sdk::serde_json::to_string(&new_public_key_string).unwrap();

        assert!(
            !keys_before
                .iter()
                .any(
                    |(public_key, _access_key)| near_sdk::serde_json::to_string(&public_key)
                        .unwrap()
                        == new_key_json_string
                ),
            "New key does not exist in access keys before being added",
        );

        s.execute_actions(
            &alice,
            &bob,
            vec![PromiseAction::AddAccessKey {
                public_key: new_public_key_string.clone(),
                allowance: NearToken::from_yoctonear(1234567890),
                receiver_id: alice.account_id().clone(),
                function_names: vec!["one".into(), "two".into(), "three".into()],
                nonce: None,
            }],
        )
        .await?;

        let keys_after = s
            .contract
            .as_account()
            .list_keys()
            .fetch_from(&s.handle.network)
            .await?
            .data;

        assert_eq!(
            keys_before.len() + 1,
            keys_after.len(),
            "There should be exactly one additional access key"
        );

        let key = keys_after
            .iter()
            .find(|(public_key, _access_key)| {
                near_sdk::serde_json::to_string(&public_key).unwrap() == new_key_json_string
            })
            .unwrap();

        let perm = match &key.1.permission {
            AccessKeyPermission::FunctionCall(fc) => fc,
            _ => panic!("Expected function call permission"),
        };

        assert_eq!(perm.allowance, Some(NearToken::from_yoctonear(1234567890)));
        assert_eq!(perm.method_names, &["one", "two", "three"]);
        assert_eq!(perm.receiver_id, alice.account_id().to_string());

        new_public_key_string
    };

    // Delete the access keys
    {
        let keys_before = s
            .contract
            .as_account()
            .list_keys()
            .fetch_from(&s.handle.network)
            .await?
            .data;

        s.execute_actions(
            &alice,
            &bob,
            vec![
                PromiseAction::DeleteKey {
                    public_key: full_access_key.clone(),
                },
                PromiseAction::DeleteKey {
                    public_key: function_call_key.clone(),
                },
            ],
        )
        .await?;

        let keys_after = s
            .contract
            .as_account()
            .list_keys()
            .fetch_from(&s.handle.network)
            .await?
            .data;

        assert_eq!(
            keys_before.len() - 2,
            keys_after.len(),
            "There should be exactly two fewer access keys"
        );

        let full_json = near_sdk::serde_json::to_string(&full_access_key).unwrap();
        let func_json = near_sdk::serde_json::to_string(&function_call_key).unwrap();

        assert!(!keys_after.iter().any(|(public_key, _access_key)| {
            let k = near_sdk::serde_json::to_string(&public_key).unwrap();
            k == full_json || k == func_json
        }));
    }

    Ok(())
}

#[tokio::test]
async fn transfer() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice").await?;
    let bob = s.setup_account("bob").await?;
    let charlie = s.setup_account("charlie").await?;

    // Send 10 NEAR to charlie
    let request_id = s
        .request(
            &alice,
            None,
            charlie.account_id(),
            [PromiseAction::Transfer {
                amount: NearToken::from_near(10),
            }],
        )
        .await?;

    assert!(!s.is_approved(request_id).await?);

    s.approve(&alice, None, request_id).await?;

    assert!(!s.is_approved(request_id).await?);

    s.approve(&bob, None, request_id).await?;

    assert!(s.is_approved(request_id).await?);

    s.approve(&charlie, None, request_id).await?;

    assert!(s.is_approved(request_id).await?);

    let balance_before = charlie
        .view()
        .fetch_from(&s.handle.network)
        .await?
        .data
        .amount;

    s.execute(&alice, None, request_id).await?;

    let balance_after = charlie
        .view()
        .fetch_from(&s.handle.network)
        .await?
        .data
        .amount;

    // charlie's balance should have increased by exactly 10 NEAR
    assert_eq!(
        balance_after.saturating_sub(balance_before),
        NearToken::from_near(10),
    );

    Ok(())
}

#[tokio::test]
async fn reflexive_xcc() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice").await?;
    let bob = s.setup_account("bob").await?;
    let charlie = s.setup_account("charlie").await?;

    let actions = vec![PromiseAction::FunctionCall {
        function_name: "private_add_one".into(),
        arguments: json!({ "value": 25 })
            .to_string()
            .as_bytes()
            .to_vec()
            .into(),
        amount: NearToken::from_yoctonear(0),
        gas: Gas::from_tgas(50),
    }];

    let request_id = s
        .request(&alice, None, s.contract.account_id(), actions)
        .await?;

    let result = s
        .double_approve_and_execute(&alice, &bob, &charlie, request_id)
        .await?
        .json::<u32>()?;

    assert_eq!(result, 26);

    Ok(())
}

#[tokio::test]
async fn external_xcc() -> TestResult<()> {
    let s = setup().await?;

    let alice = s.setup_account("alice").await?;
    let bob = s.setup_account("bob").await?;
    let charlie = s.setup_account("charlie").await?;

    let second_contract = s
        .handle
        .make_contract(
            "cross_target",
            "cross_target",
            json!({ "owner_id": s.contract.account_id() }),
        )
        .await?;

    let actions = vec![PromiseAction::FunctionCall {
        function_name: "set_value".into(),
        arguments: json!({ "value": "Hello, world!" })
            .to_string()
            .as_bytes()
            .to_vec()
            .into(),
        amount: NearToken::from_yoctonear(0),
        gas: Gas::from_tgas(50),
    }];

    let request_id = s
        .request(&alice, None, second_contract.account_id(), actions)
        .await?;

    s.approve(&alice, None, request_id).await?;
    s.approve(&bob, None, request_id).await?;

    let value_before = second_contract
        .call_function("get_value", json!({}))
        .read_only::<String>()
        .fetch_from(&s.handle.network)
        .await?
        .data;

    assert_eq!(value_before, "");

    let calls_before = second_contract
        .call_function("get_calls", json!({}))
        .read_only::<u32>()
        .fetch_from(&s.handle.network)
        .await?
        .data;

    assert_eq!(calls_before, 0);

    s.execute(&charlie, None, request_id).await?;

    let value_after = second_contract
        .call_function("get_value", json!({}))
        .read_only::<String>()
        .fetch_from(&s.handle.network)
        .await?
        .data;

    assert_eq!(value_after, "Hello, world!");

    let calls_after = second_contract
        .call_function("get_calls", json!({}))
        .read_only::<u32>()
        .fetch_from(&s.handle.network)
        .await?
        .data;

    assert_eq!(calls_after, 1);

    Ok(())
}
