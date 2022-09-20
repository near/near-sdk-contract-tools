use near_contract_tools::{standard::nep141::*, Nep141};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::Vector,
    env,
    json_types::U128,
    log, near_bindgen,
    test_utils::VMContextBuilder,
    testing_env, AccountId, PromiseOrValue,
};

#[derive(Nep141, BorshDeserialize, BorshSerialize)]
#[near_bindgen]
struct FungibleToken {
    pub transfers: Vector<Nep141Transfer>,
    pub hooks: Vector<String>,
}

#[derive(Default)]
struct HookState {
    pub storage_usage_start: u64,
}

impl Nep141Hook<HookState> for FungibleToken {
    fn before_transfer(&mut self, transfer: &Nep141Transfer) -> HookState {
        self.transfers.push(transfer);
        self.hooks.push(&"before_transfer".to_string());

        HookState {
            storage_usage_start: env::storage_usage(),
        }
    }

    fn after_transfer(&mut self, _transfer: &Nep141Transfer, state: HookState) {
        self.hooks.push(&"after_transfer".to_string());
        println!(
            "Storage delta: {}",
            env::storage_usage() - state.storage_usage_start
        );
    }
}

#[near_bindgen]
struct FungibleTokenReceiver {
    pub log: Vector<(String, u128)>,
}

impl near_contract_tools::standard::nep141::Nep141Receiver for FungibleTokenReceiver {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let used_amount: u128 = amount.0 / 2;

        let out = format!("ft_on_transfer[from={sender_id}, used={used_amount}]");
        log!(&out);
        println!("{out}");

        self.log.push(&(msg, amount.0));

        PromiseOrValue::Value(U128(used_amount))
    }
}

// TODO: transfer_call testing (not possible without workspaces-rs or something
//  like that, and workspaces-rs doesn't work on macOS)
#[test]
fn nep141_transfer() {
    let mut ft = FungibleToken {
        transfers: Vector::new(b"t"),
        hooks: Vector::new(b"h"),
    };

    let alice: AccountId = "alice".parse().unwrap();
    let bob: AccountId = "bob".parse().unwrap();

    assert_eq!(ft.ft_balance_of(alice.clone()).0, 0);
    assert_eq!(ft.ft_balance_of(bob.clone()).0, 0);
    assert_eq!(ft.ft_total_supply().0, 0);

    ft.deposit_unchecked(&alice, 100);
    ft.deposit_unchecked(&bob, 20);

    assert_eq!(ft.transfers.pop(), None);
    assert_eq!(ft.ft_balance_of(alice.clone()).0, 100);
    assert_eq!(ft.ft_balance_of(bob.clone()).0, 20);
    assert_eq!(ft.ft_total_supply().0, 120);

    let context = VMContextBuilder::new()
        .predecessor_account_id(alice.clone())
        .attached_deposit(1)
        .build();

    testing_env!(context);

    ft.ft_transfer(bob.clone(), 50.into(), None);

    assert_eq!(
        ft.transfers.pop(),
        Some(Nep141Transfer {
            sender_id: alice.clone(),
            receiver_id: bob.clone(),
            amount: 50,
            memo: None,
            msg: None,
        })
    );

    let expected_hook_execution_order = vec!["before_transfer", "after_transfer"];
    let actual_hook_execution_order = ft.hooks.to_vec();
    assert_eq!(expected_hook_execution_order, actual_hook_execution_order);

    assert_eq!(ft.ft_balance_of(alice.clone()).0, 50);
    assert_eq!(ft.ft_balance_of(bob.clone()).0, 70);
    assert_eq!(ft.ft_total_supply().0, 120);
}
