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

#[derive(BorshDeserialize, BorshSerialize, PartialEq, Debug)]
struct TransferRecord {
    pub sender_id: AccountId,
    pub receiver_id: AccountId,
    pub amount: u128,
    pub memo: Option<String>,
}

#[derive(Nep141)]
// #[nep141(hook)]
#[near_bindgen]
struct FungibleToken {
    pub transfers: Vector<TransferRecord>,
}

impl FungibleToken {
    fn hook_vector() -> Vector<String> {
        env::storage_read(b"h")
            .map(|s| Vector::<String>::try_from_slice(&s).unwrap())
            .unwrap_or_else(|| Vector::new(b"h" as &[u8]))
    }

    fn push_hook(hook: &str) {
        let mut v = Self::hook_vector();
        v.push(&hook.to_string());
        env::storage_write(b"h", &v.try_to_vec().unwrap());
    }

    fn before_transfer() {
        Self::push_hook("before_transfer");
    }

    fn before_transfer_plain() {
        Self::push_hook("before_transfer_plain");
    }

    fn before_transfer_call() {
        Self::push_hook("before_transfer_call");
    }

    fn after_transfer() {
        Self::push_hook("after_transfer");
    }

    fn after_transfer_plain() {
        Self::push_hook("after_transfer_plain");
    }

    fn after_transfer_call() {
        Self::push_hook("after_transfer_call");
    }

    fn before_transfer_args(
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

        Self::push_hook("before_transfer_args");
    }

    fn before_transfer_plain_args(
        &mut self,
        _: &AccountId,
        _: &AccountId,
        _: u128,
        _: Option<&str>,
    ) {
        Self::push_hook("before_transfer_plain_args");
    }

    fn before_transfer_call_args(
        &mut self,
        _: &AccountId,
        _: &AccountId,
        _: u128,
        _: Option<&str>,
        _: &str,
    ) {
        Self::push_hook("before_transfer_call_args");
    }

    fn after_transfer_args(&mut self, _: &AccountId, _: &AccountId, _: u128, _: Option<&str>) {
        Self::push_hook("after_transfer_args");
    }

    fn after_transfer_plain_args(
        &mut self,
        _: &AccountId,
        _: &AccountId,
        _: u128,
        _: Option<&str>,
    ) {
        Self::push_hook("after_transfer_plain_args");
    }

    fn after_transfer_call_args(
        &mut self,
        _: &AccountId,
        _: &AccountId,
        _: u128,
        _: Option<&str>,
        _: &str,
    ) {
        Self::push_hook("after_transfer_call_args");
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
        "before_transfer_plain_args",
        "before_transfer_args",
        "before_transfer_plain",
        "before_transfer",
        "after_transfer_plain_args",
        "after_transfer_args",
        "after_transfer_plain",
        "after_transfer",
    ];
    let actual_hook_execution_order = FungibleToken::hook_vector().to_vec();
    assert_eq!(expected_hook_execution_order, actual_hook_execution_order);

    assert_eq!(ft.ft_balance_of(alice.clone()).0, 50);
    assert_eq!(ft.ft_balance_of(bob.clone()).0, 70);
    assert_eq!(ft.ft_total_supply().0, 120);
}
