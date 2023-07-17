use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen, store, AccountId,
};
use near_sdk_contract_tools::{standard::nep171::*, Nep171};

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

#[derive(Nep171, BorshDeserialize, BorshSerialize)]
#[near_bindgen]
struct NonFungibleToken {
    pub before_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
    pub after_nft_transfer_balance_record: store::Vector<Option<TokenRecord>>,
}

impl Nep171Hook for NonFungibleToken {
    fn before_nft_transfer(&mut self, transfer: &Nep171Transfer) {
        let token = Nep171::nft_token(self, transfer.token_id.clone());
        self.before_nft_transfer_balance_record
            .push(token.map(Into::into));
    }

    fn after_nft_transfer(&mut self, transfer: &Nep171Transfer, _state: ()) {
        let token = Nep171::nft_token(self, transfer.token_id.clone());
        self.after_nft_transfer_balance_record
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

    pub fn mint(&mut self, token_id: TokenId, owner_id: AccountId) {
        Nep171Controller::mint(self, &[token_id], &owner_id).unwrap_or_else(|e| {
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
