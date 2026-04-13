#![allow(dead_code)]

use near_sdk::{AccountId, PanicOnDefault, env, near, store};
use near_sdk_contract_tools::{hook::Hook, nft::*};

mod hooks;
mod manual_integration;
mod no_hooks;
mod non_fungible_token;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[near]
struct TokenRecord {
    owner_id: AccountId,
    token_id: TokenId,
}

impl From<Token> for TokenRecord {
    fn from(token: Token) -> Self {
        Self {
            owner_id: token.owner_id,
            token_id: token.token_id,
        }
    }
}

mod full_no_hooks {
    use near_sdk::NearToken;

    use super::*;

    #[derive(NonFungibleToken, PanicOnDefault)]
    #[near(contract_state)]
    struct NonFungibleTokenNoHooks {
        pub before_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
        pub after_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
    }

    #[test]
    fn nft_no_hooks() {
        let mut n = NonFungibleTokenNoHooks {
            before_nft_transfer_balance_record: store::Vector::new(b"a"),
            after_nft_transfer_balance_record: store::Vector::new(b"b"),
        };

        let token_id = "token1".to_string();
        let alice: AccountId = "alice".parse().unwrap();

        Nep145Controller::deposit_to_storage_account(&mut n, &alice, NearToken::from_near(1))
            .unwrap();

        n.mint_with_metadata(&token_id, &alice, &TokenMetadata::new().title("Title"))
            .unwrap();

        let nft_tok = n.nft_token(token_id);
        dbg!(nft_tok);
    }
}

#[derive(Nep171, PanicOnDefault)]
#[nep171(transfer_hook = "Self")]
#[near(contract_state)]
struct NonFungibleToken {
    pub before_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
    pub after_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
}

impl Hook<NonFungibleToken, Nep171Transfer<'_>> for NonFungibleToken {
    fn hook<R>(
        contract: &mut NonFungibleToken,
        args: &Nep171Transfer<'_>,
        f: impl FnOnce(&mut NonFungibleToken) -> R,
    ) -> R {
        let before_nft_transfer = contract.nft_token(args.token_id.clone()).map(Into::into);
        contract
            .before_nft_transfer_balance_record
            .push(before_nft_transfer);
        let r = f(contract);
        let after_nft_transfer = contract.nft_token(args.token_id.clone()).map(Into::into);
        contract
            .after_nft_transfer_balance_record
            .push(after_nft_transfer);
        r
    }
}

#[near]
impl NonFungibleToken {
    #[init]
    pub fn new() -> Self {
        Self {
            before_nft_transfer_balance_record: store::Vector::new(b"b"),
            after_nft_transfer_balance_record: store::Vector::new(b"a"),
        }
    }

    pub fn mint(&mut self, token_id: TokenId, receiver_id: AccountId) {
        Nep171Controller::mint(self, &Nep171Mint::new(vec![token_id], receiver_id)).unwrap_or_else(
            |e| {
                env::panic_str(&format!("Mint failed: {e:?}"));
            },
        );
    }
}

mod tests {
    use near_sdk::{
        AccountId, NearToken,
        test_utils::{VMContextBuilder, get_logs},
        testing_env,
    };
    use near_sdk_contract_tools::standard::{
        nep171::{
            Nep171,
            event::{Nep171Event, NftTransferLog},
        },
        nep297::Event,
    };

    use super::*;

    #[test]
    fn hook_execution_success() {
        let mut contract = NonFungibleToken::new();
        let token_id = "token1";
        let account_alice: AccountId = "alice.near".parse().unwrap();
        let account_bob: AccountId = "bob.near".parse().unwrap();

        contract.mint(token_id.to_string(), account_alice.clone());

        assert_eq!(
            contract.before_nft_transfer_balance_record.get(0),
            None,
            "before_nft_transfer_balance_record should be empty",
        );
        assert_eq!(
            contract.after_nft_transfer_balance_record.get(0),
            None,
            "after_nft_transfer_balance_record should be empty",
        );

        testing_env!(
            VMContextBuilder::new()
                .predecessor_account_id(account_alice.clone())
                .attached_deposit(NearToken::from_yoctonear(1u128))
                .build()
        );

        contract.nft_transfer(account_bob.clone(), token_id.to_string(), None, None);

        assert_eq!(
            contract.before_nft_transfer_balance_record.get(0),
            Some(&Some(TokenRecord {
                owner_id: account_alice.clone(),
                token_id: token_id.to_string(),
            })),
            "before_nft_transfer_balance_record should contain the token record for the original owner before transferring",
        );
        assert_eq!(
            contract.after_nft_transfer_balance_record.get(0),
            Some(&Some(TokenRecord {
                owner_id: account_bob.clone(),
                token_id: token_id.to_string(),
            })),
            "after_nft_transfer_balance_record should contain the token record for the new owner after transferring",
        );

        assert_eq!(
            get_logs(),
            vec![
                Nep171Event::NftTransfer(vec![NftTransferLog {
                    memo: None,
                    authorized_id: None,
                    old_owner_id: account_alice.into(),
                    new_owner_id: account_bob.into(),
                    token_ids: vec![token_id.into()]
                }])
                .to_event_string()
            ]
        );
    }
}
