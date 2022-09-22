use near_contract_tools::{
    standard::nep141::{Nep141, Nep141Controller},
    FungibleToken,
};
use near_sdk::{
    json_types::Base64VecU8, near_bindgen, test_utils::VMContextBuilder, testing_env, AccountId,
};
use std::borrow::Cow;

#[derive(FungibleToken)]
#[fungible_token(
    name = "My Fungible Token",
    symbol = "MYFT",
    decimals = 18,
    icon = "https://example.com/icon.png",
    reference = "https://example.com/metadata.json",
    reference_hash = "YXNkZg==",
    no_hooks
)]
#[near_bindgen]
struct MyFungibleTokenContract {}

#[test]
fn fungible_token_transfer() {
    let mut ft = MyFungibleTokenContract {};

    let alice: AccountId = "alice".parse().unwrap();
    let bob: AccountId = "bob".parse().unwrap();

    assert_eq!(ft.ft_balance_of(alice.clone()).0, 0);
    assert_eq!(ft.ft_balance_of(bob.clone()).0, 0);
    assert_eq!(ft.ft_total_supply().0, 0);

    ft.deposit_unchecked(&alice, 100);
    ft.deposit_unchecked(&bob, 20);

    assert_eq!(ft.ft_balance_of(alice.clone()).0, 100);
    assert_eq!(ft.ft_balance_of(bob.clone()).0, 20);
    assert_eq!(ft.ft_total_supply().0, 120);

    let context = VMContextBuilder::new()
        .predecessor_account_id(alice.clone())
        .attached_deposit(1)
        .build();

    testing_env!(context);

    ft.ft_transfer(bob.clone(), 50.into(), None);

    assert_eq!(ft.ft_balance_of(alice.clone()).0, 50);
    assert_eq!(ft.ft_balance_of(bob.clone()).0, 70);
    assert_eq!(ft.ft_total_supply().0, 120);
}

#[test]
fn metadata() {
    let ft = MyFungibleTokenContract {};
    let meta = ft.ft_metadata();

    assert_eq!(meta.decimals, 18);
    assert_eq!(meta.name, "My Fungible Token");
    assert_eq!(meta.symbol, "MYFT");
    assert_eq!(meta.icon, Some("https://example.com/icon.png".into()));
    assert_eq!(
        meta.reference,
        Some("https://example.com/metadata.json".into())
    );
    assert_eq!(
        meta.reference_hash,
        Some(Cow::Owned(Base64VecU8::from([97, 115, 100, 102].to_vec())))
    );
}
