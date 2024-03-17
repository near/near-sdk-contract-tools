#![allow(dead_code)]

compat_use_borsh!();
use near_sdk::{env, near_bindgen, store, AccountId};
use near_sdk_contract_tools::{
    compat_derive_borsh, compat_near, compat_near_to_u128, compat_use_borsh, hook::Hook, nft::*,
};

mod hooks;
mod manual_integration;
mod no_hooks;
mod non_fungible_token;

compat_derive_borsh! {
    #[derive(Debug, Clone, PartialEq, PartialOrd)]
    struct TokenRecord {
        owner_id: AccountId,
        token_id: TokenId,
    }
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
    use super::*;

    compat_derive_borsh! {
        #[derive(NonFungibleToken)]
        #[near_bindgen]
        struct NonFungibleTokenNoHooks {
            pub before_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
            pub after_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
        }
    }

    #[test]
    fn nft_no_hooks() {
        let mut n = NonFungibleTokenNoHooks {
            before_nft_transfer_balance_record: store::Vector::new(b"a"),
            after_nft_transfer_balance_record: store::Vector::new(b"b"),
        };

        let token_id = "token1".to_string();
        let alice: AccountId = "alice".parse().unwrap();

        Nep145Controller::deposit_to_storage_account(
            &mut n,
            &alice,
            compat_near_to_u128!(compat_near!(1u128)).into(),
        )
        .unwrap();

        n.mint_with_metadata(token_id.clone(), alice, TokenMetadata::new().title("Title"))
            .unwrap();

        let nft_tok = n.nft_token(token_id);
        dbg!(nft_tok);
    }
}

compat_derive_borsh! {
    #[derive(Nep171)]
    #[nep171(transfer_hook = "Self")]
    #[near_bindgen]
    struct NonFungibleToken {
        pub before_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
        pub after_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
    }
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

#[near_bindgen]
impl NonFungibleToken {
    #[init]
    pub fn new() -> Self {
        Self {
            before_nft_transfer_balance_record: store::Vector::new(b"b"),
            after_nft_transfer_balance_record: store::Vector::new(b"a"),
        }
    }

    pub fn mint(&mut self, token_id: TokenId, receiver_id: AccountId) {
        let action = Nep171Mint {
            token_ids: &[token_id],
            receiver_id: &receiver_id,
            memo: None,
        };
        Nep171Controller::mint(self, &action).unwrap_or_else(|e| {
            env::panic_str(&format!("Mint failed: {e:?}"));
        });
    }
}

mod tests {
    use near_sdk::{
        test_utils::{get_logs, VMContextBuilder},
        testing_env, AccountId,
    };
    use near_sdk_contract_tools::{
        compat_yoctonear,
        standard::{
            nep171::{
                event::{Nep171Event, NftTransferLog},
                Nep171,
            },
            nep297::Event,
        },
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

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(account_alice.clone())
            .attached_deposit(compat_yoctonear!(1u128))
            .build());

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
            vec![Nep171Event::NftTransfer(vec![NftTransferLog {
                memo: None,
                authorized_id: None,
                old_owner_id: account_alice.clone(),
                new_owner_id: account_bob.clone(),
                token_ids: vec![token_id.to_string()]
            }])
            .to_event_string()]
        );
    }
}
