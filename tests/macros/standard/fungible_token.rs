use near_sdk::{PanicOnDefault, json_types::Base64VecU8, near};
use near_sdk_contract_tools::ft::*;

#[derive(FungibleToken, PanicOnDefault)]
#[near(contract_state)]
struct MyFungibleTokenContract {}

#[near]
impl MyFungibleTokenContract {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self {};

        contract.set_metadata(
            &ContractMetadata::new("My Fungible Token", "MYFT", 24)
                .icon("https://example.com/icon.png")
                .reference("https://example.com/metadata.json")
                .reference_hash(Base64VecU8::from([97, 115, 100, 102].to_vec())),
        );

        contract
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::{AccountId, NearToken, test_utils::VMContextBuilder, testing_env};

    #[test]
    fn fungible_token_transfer() {
        let mut ft = MyFungibleTokenContract::new();

        let alice: AccountId = "alice".parse().unwrap();
        let bob: AccountId = "bob".parse().unwrap();

        let context = VMContextBuilder::new()
            .predecessor_account_id(alice.clone())
            .attached_deposit(NearToken::from_near(1u128).saturating_div(100))
            .build();
        testing_env!(context);
        ft.storage_deposit(None, None);

        let context = VMContextBuilder::new()
            .predecessor_account_id(bob.clone())
            .attached_deposit(NearToken::from_near(1u128).saturating_div(100))
            .build();
        testing_env!(context);
        ft.storage_deposit(None, None);

        assert_eq!(ft.ft_balance_of(alice.clone()).0, 0);
        assert_eq!(ft.ft_balance_of(bob.clone()).0, 0);
        assert_eq!(ft.ft_total_supply().0, 0);

        ft.deposit_unchecked(&alice, 100).unwrap();
        ft.deposit_unchecked(&bob, 20).unwrap();

        assert_eq!(ft.ft_balance_of(alice.clone()).0, 100);
        assert_eq!(ft.ft_balance_of(bob.clone()).0, 20);
        assert_eq!(ft.ft_total_supply().0, 120);

        let context = VMContextBuilder::new()
            .predecessor_account_id(alice.clone())
            .attached_deposit(NearToken::from_yoctonear(1u128))
            .build();

        testing_env!(context);

        ft.ft_transfer(bob.clone(), 50.into(), None);

        assert_eq!(ft.ft_balance_of(alice.clone()).0, 50);
        assert_eq!(ft.ft_balance_of(bob.clone()).0, 70);
        assert_eq!(ft.ft_total_supply().0, 120);
    }

    #[test]
    fn metadata() {
        let ft = MyFungibleTokenContract::new();
        let meta = ft.ft_metadata();

        assert_eq!(meta.decimals, 24);
        assert_eq!(meta.name, "My Fungible Token");
        assert_eq!(meta.symbol, "MYFT");
        assert_eq!(meta.icon, Some("https://example.com/icon.png".into()));
        assert_eq!(
            meta.reference,
            Some("https://example.com/metadata.json".into())
        );
        assert_eq!(
            meta.reference_hash,
            Some(Base64VecU8::from([97, 115, 100, 102].to_vec()))
        );
    }
}
