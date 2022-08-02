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
#[nep141(on_transfer = "Self::on_transfer")]
#[near_bindgen]
struct FungibleToken {
    pub transfers: Vector<TransferRecord>,
}

impl FungibleToken {
    fn on_transfer(
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
        })
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

#[test]
fn nep141_transfer() {
    let mut ft = FungibleToken {
        transfers: Vector::new(b"t"),
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
    assert_eq!(ft.ft_balance_of(alice.clone()).0, 50);
    assert_eq!(ft.ft_balance_of(bob.clone()).0, 70);
    assert_eq!(ft.ft_total_supply().0, 120);
}
