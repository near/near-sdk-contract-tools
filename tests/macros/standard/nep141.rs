use near_contract_tools::{standard::nep141::*, Nep141};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::Vector,
    json_types::U128,
    log, near_bindgen,
    test_utils::VMContextBuilder,
    testing_env, AccountId, PromiseOrValue,
};

#[derive(BorshDeserialize, BorshSerialize, PartialEq, Debug)]
struct TransferRecord {
    pub sender_id: AccountId,
    pub receiver_id: AccountId,
    pub amount: u128,
    pub memo: Option<String>,
}

#[derive(Nep141)]
#[nep141(
    before_transfer = "Self::before_transfer",
    before_transfer_plain = "Self::before_transfer_plain",
    before_transfer_call = "Self::before_transfer_call",
    after_transfer = "Self::after_transfer",
    after_transfer_plain = "Self::after_transfer_plain",
    after_transfer_call = "Self::after_transfer_call"
)]
#[near_bindgen]
struct FungibleToken {
    pub transfers: Vector<TransferRecord>,
    pub hooks: Vector<String>,
}

impl FungibleToken {
    fn before_transfer(
        &mut self,
        sender_id: &AccountId,
        receiver_id: &AccountId,
        amount: u128,
        memo: Option<&str>,
    ) {
        self.transfers.push(&TransferRecord {
            sender_id: sender_id.clone(),
            receiver_id: receiver_id.clone(),
            amount,
            memo: memo.map(|s| s.to_string()),
        });

        self.hooks.push(&"before_transfer".to_string());
    }

    fn before_transfer_plain(&mut self, _: &AccountId, _: &AccountId, _: u128, _: Option<&str>) {
        self.hooks.push(&"before_transfer_plain".to_string());
    }

    fn before_transfer_call(
        &mut self,
        _: &AccountId,
        _: &AccountId,
        _: u128,
        _: Option<&str>,
        _: &str,
    ) {
        self.hooks.push(&"before_transfer_call".to_string());
    }

    fn after_transfer(&mut self, _: &AccountId, _: &AccountId, _: u128, _: Option<&str>) {
        self.hooks.push(&"after_transfer".to_string());
    }

    fn after_transfer_plain(&mut self, _: &AccountId, _: &AccountId, _: u128, _: Option<&str>) {
        self.hooks.push(&"after_transfer_plain".to_string());
    }

    fn after_transfer_call(
        &mut self,
        _: &AccountId,
        _: &AccountId,
        _: u128,
        _: Option<&str>,
        _: &str,
    ) {
        self.hooks.push(&"after_transfer_call".to_string());
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

    ft.internal_deposit(&alice, 100);
    ft.internal_deposit(&bob, 20);

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
        Some(TransferRecord {
            sender_id: alice.clone(),
            receiver_id: bob.clone(),
            amount: 50,
            memo: None
        })
    );

    let expected_hook_execution_order = vec![
        "before_transfer_plain",
        "before_transfer",
        "after_transfer_plain",
        "after_transfer",
    ];
    let actual_hook_execution_order = ft.hooks.to_vec();
    assert_eq!(expected_hook_execution_order, actual_hook_execution_order);

    assert_eq!(ft.ft_balance_of(alice.clone()).0, 50);
    assert_eq!(ft.ft_balance_of(bob.clone()).0, 70);
    assert_eq!(ft.ft_total_supply().0, 120);
}
