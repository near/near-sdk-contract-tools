#![allow(dead_code)]

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen, store, AccountId,
};
use near_sdk_contract_tools::{
    hook::Hook,
    standard::{
        nep171::*,
        nep177::{Nep177Controller, TokenMetadata},
    },
    Nep171, NonFungibleToken,
};

mod hooks;
mod manual_integration;
mod no_hooks;
mod non_fungible_token;

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, PartialEq, PartialOrd)]
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

#[derive(NonFungibleToken, BorshDeserialize, BorshSerialize)]
#[non_fungible_token(no_approval_hooks)]
#[near_bindgen]
struct NonFungibleTokenNoHooks {
    pub before_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
    pub after_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
}

#[test]
fn t() {
    let mut n = NonFungibleTokenNoHooks {
        before_nft_transfer_balance_record: store::Vector::new(b"a"),
        after_nft_transfer_balance_record: store::Vector::new(b"b"),
    };

    let token_id = "token1".to_string();

    n.mint_with_metadata(
        token_id.clone(),
        "alice".parse().unwrap(),
        TokenMetadata {
            title: Some("Title".to_string()),
            description: None,
            media: None,
            media_hash: None,
            copies: None,
            issued_at: None,
            expires_at: None,
            starts_at: None,
            updated_at: None,
            extra: None,
            reference: None,
            reference_hash: None,
        },
    )
    .unwrap();

    let nft_tok = n.nft_token(token_id);
    dbg!(nft_tok);
}

#[derive(Nep171, BorshDeserialize, BorshSerialize)]
#[nep171(transfer_hook = "Self")]
#[near_bindgen]
struct NonFungibleToken {
    pub before_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
    pub after_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
}

impl Hook<NonFungibleToken, Nep171Transfer<'_>> for NonFungibleToken {
    type State = Option<TokenRecord>;

    fn before(contract: &Self, transfer: &Nep171Transfer, state: &mut Self::State) {
        let token = Nep171::nft_token(contract, transfer.token_id.clone());
        *state = token.map(Into::into);
    }

    fn after(
        contract: &mut Self,
        transfer: &Nep171Transfer,
        before_nft_transfer: Option<TokenRecord>,
    ) {
        let token = Nep171::nft_token(contract, transfer.token_id.clone());
        contract
            .before_nft_transfer_balance_record
            .push(before_nft_transfer);
        contract
            .after_nft_transfer_balance_record
            .push(token.map(Into::into));
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
    use near_sdk_contract_tools::standard::{nep171::Nep171, nep297::Event};

    use super::NonFungibleToken;

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
            .attached_deposit(1)
            .build());

        contract.nft_transfer(account_bob.clone(), token_id.to_string(), None, None);

        assert_eq!(
            contract.before_nft_transfer_balance_record.get(0),
            Some(&Some(super::TokenRecord {
                owner_id: account_alice.clone(),
                token_id: token_id.to_string(),
            })),
            "before_nft_transfer_balance_record should contain the token record for the original owner before transferring",
        );
        assert_eq!(
            contract.after_nft_transfer_balance_record.get(0),
            Some(&Some(super::TokenRecord {
                owner_id: account_bob.clone(),
                token_id: token_id.to_string(),
            })),
            "after_nft_transfer_balance_record should contain the token record for the new owner after transferring",
        );

        assert_eq!(
            get_logs(),
            vec![
                super::Nep171Event::NftTransfer(vec![super::event::NftTransferLog {
                    memo: None,
                    authorized_id: None,
                    old_owner_id: account_alice.clone(),
                    new_owner_id: account_bob.clone(),
                    token_ids: vec![token_id.to_string()]
                }])
                .to_event_string()
            ]
        );
    }
}
