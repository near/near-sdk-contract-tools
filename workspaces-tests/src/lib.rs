#![allow(missing_docs)]

use std::{sync::Arc, time::Duration};

use cargo_near_build::camino::{Utf8Path, Utf8PathBuf};
use near_api::{Account, Contract, NetworkConfig};
use near_sandbox::Sandbox;
use near_sdk::{AccountId, NearToken, serde::Serialize};
use testresult::TestResult;

pub const ONE_YOCTO: NearToken = NearToken::from_yoctonear(1);
pub const Y: Option<NearToken> = Some(NearToken::from_yoctonear(1));
pub const ONE_NEAR: NearToken = NearToken::from_near(1);

#[macro_export]
macro_rules! read_only {
    (fn $name:ident ( $( $arg:ident : $arg_t:ty ),* ) -> $ret_t:ty) => {
        async fn $name(
            &self,
            $( $arg : impl Into<$arg_t> ),*
        ) -> ::testresult::TestResult<$ret_t> {
            #[derive(::near_sdk::serde::Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Args {
                $( $arg : $arg_t ),*
            }

            Ok(self
                .contract
                .call_function(stringify!($name), Args { $( $arg : $arg.into() ),* })
                .read_only::<$ret_t>()
                .fetch_from(&self.handle.network)
                .await?
                .data)
        }
    };
}

#[macro_export]
macro_rules! transaction {
    (fn $name:ident ( $( $arg:ident : $arg_t:ty ),* ) ) => {
        #[allow(clippy::too_many_arguments)]
        async fn $name(
            &self,
            account: &::near_api::Account,
            deposit: Option<::near_sdk::NearToken>,
            $( $arg : impl Into<$arg_t> ),*
        ) -> ::testresult::TestResult<::near_api::types::transaction::result::ExecutionResult<::near_api::types::transaction::result::Value>> {
            #[derive(::near_sdk::serde::Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Args {
                $( $arg : $arg_t ),*
            }

            let result = self
                .contract
                .call_function(stringify!($name), Args { $( $arg : $arg.into() ),* })
                .transaction()
                .deposit(deposit.unwrap_or(::near_sdk::NearToken::from_yoctonear(0)))
                .with_signer(account.account_id().clone(), self.handle.default_signer())
                .send_to(&self.handle.network)
                .await?
                .into_result()?;

            Ok(result)
        }
    };

    (fn $name:ident ( $( $arg:ident : $arg_t:ty ),* ) -> $ret_t:ty) => {
        #[allow(clippy::too_many_arguments)]
        async fn $name(
            &self,
            account: &::near_api::Account,
            deposit: Option<::near_sdk::NearToken>,
            $( $arg : impl Into<$arg_t> ),*
        ) -> ::testresult::TestResult<$ret_t> {
            #[derive(::near_sdk::serde::Serialize)]
            #[serde(crate = "::near_sdk::serde")]
            struct Args {
                $( $arg : $arg_t ),*
            }

            Ok(self
                .contract
                .call_function(stringify!($name), Args { $( $arg : $arg.into() ),* })
                .transaction()
                .deposit(deposit.unwrap_or(::near_sdk::NearToken::from_yoctonear(0)))
                .with_signer(account.account_id().clone(), self.handle.default_signer())
                .send_to(&self.handle.network)
                .await?
                .into_result()?
                .json()?)
        }
    };
}

pub struct Handle {
    pub network: NetworkConfig,
    pub sandbox: Sandbox,
}

impl Handle {
    pub async fn new() -> Self {
        let sandbox = Sandbox::start_sandbox().await.unwrap();
        let network = NetworkConfig::from_rpc_url("sandbox", sandbox.rpc_addr.parse().unwrap());
        Self { sandbox, network }
    }

    pub fn default_signer(&self) -> Arc<near_api::Signer> {
        near_api::Signer::from_secret_key(
            near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY
                .parse()
                .unwrap(),
        )
        .unwrap()
    }

    pub async fn deploy(
        &self,
        account_id: AccountId,
        wasm: &[u8],
        args: impl Serialize,
    ) -> TestResult<()> {
        near_api::Contract::deploy(account_id)
            .use_code(wasm.to_vec())
            .with_init_call("new", args)
            .unwrap()
            .max_gas()
            .with_signer(self.default_signer())
            .send_to(&self.network)
            .await?
            .assert_success();
        Ok(())
    }

    pub async fn load_wasm(&self, contract: impl Into<String>) -> TestResult<Vec<u8>> {
        let contract = contract.into();
        let bytes = if std::env::var("NO_BUILD_TEST_CONTRACTS").is_ok() {
            let wasm_path = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../")
                .join("target/near/")
                .join(&contract)
                .join(format!("{contract}.wasm"))
                .canonicalize_utf8()?;
            eprintln!("Loading wasm from {wasm_path}");
            // near sandbox crashes with "access key ed25519:5BGSaf6YjVm7565VzWQHNxoyEjwr3jUpRJSGjREvU9dB does not exist while viewing" if we don't sleep here.
            tokio::time::sleep(Duration::from_secs(1)).await;
            std::fs::read(wasm_path)?
        } else {
            let manifest_path = Utf8Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../")
                .join("test-contracts/")
                .join(&contract)
                .join("Cargo.toml")
                .canonicalize_utf8()?;
            eprintln!("Loading manifest from {manifest_path}");
            self.build(manifest_path).await?
        };
        eprintln!("Bytes len: {}", bytes.len());
        Ok(bytes)
    }

    pub async fn build(&self, manifest_path: impl Into<Utf8PathBuf>) -> TestResult<Vec<u8>> {
        let path = cargo_near_build::build_with_cli(
            cargo_near_build::BuildOpts::builder()
                .manifest_path(manifest_path)
                .build(),
        )
        .unwrap();
        Ok(std::fs::read(path)?)
    }

    pub async fn make_contract(
        &self,
        name: impl Into<String>,
        contract: impl Into<String>,
        args: impl Serialize,
    ) -> TestResult<Contract> {
        let wasm: &[u8] = &self.load_wasm(contract).await?;
        let account = self.make_account(name).await?;
        Contract::deploy(account.account_id().clone())
            .use_code(wasm.to_vec())
            .with_init_call("new", args)?
            .max_gas()
            .with_signer(self.default_signer())
            .send_to(&self.network)
            .await?
            .assert_success();
        Ok(account.as_contract())
    }

    pub async fn make_account(&self, name: impl Into<String>) -> TestResult<Account> {
        let account_id: AccountId = name.into().parse().unwrap();
        self.sandbox
            .create_account(account_id.clone())
            .initial_balance(NearToken::from_near(100))
            .send()
            .await?;
        Ok(Account(account_id))
    }
}

pub struct Setup {
    pub contract: Contract,
    pub accounts: Vec<Account>,
}
