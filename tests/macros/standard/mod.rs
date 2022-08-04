pub mod fungible_token;
pub mod nep141;
pub mod nep148;

mod t {
    use near_contract_tools::{
        pause::Pause,
        standard::nep141::{Nep141, Nep141Controller, Nep141Hook, Nep141Transfer},
        FungibleToken, Pause,
    };
    use near_sdk::{
        borsh::{self, BorshDeserialize, BorshSerialize},
        env, near_bindgen,
        test_utils::VMContextBuilder,
        testing_env, AccountId,
    };

    #[derive(FungibleToken, Pause, BorshDeserialize, BorshSerialize)]
    #[fungible_token(name = "Pausable Fungible Token", symbol = "PFT", decimals = 18)]
    #[near_bindgen]
    struct Contract {
        #[borsh_skip]
        nep141_hook: HookState,
    }

    #[derive(Default)]
    struct HookState {
        pub storage_usage_start: u64,
    }

    impl Nep141Hook<Contract> for HookState {
        fn before_transfer(contract: &mut Contract, _transfer: &Nep141Transfer) {
            contract.nep141_hook.storage_usage_start = env::storage_usage();
        }

        fn after_transfer(contract: &mut Contract, _transfer: &Nep141Transfer) {
            println!(
                "Storage delta: {}",
                env::storage_usage() - contract.nep141_hook.storage_usage_start
            );
        }
    }

    #[test]
    fn test() {
        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob_account".parse().unwrap();

        let mut c = Contract {
            nep141_hook: Default::default(),
        };

        c.internal_deposit(&alice, 100);

        let context = VMContextBuilder::new()
            .attached_deposit(1)
            .predecessor_account_id(alice.clone())
            .build();

        testing_env!(context);

        c.ft_transfer(bob.clone(), 50.into(), None);
    }
}
