use near_sdk::{
    AccountId, NearToken, PromiseOrValue, env, json_types::U128, log, near, serde_json,
};
use near_sdk_contract_tools::mt::*;

#[derive(Default)]
#[near(contract_state)]
pub struct Contract {}

#[near]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }

    #[private]
    pub fn return_value(&self, value: Vec<U128>) -> Vec<U128> {
        value
    }
}

#[near]
impl Nep245Receiver for Contract {
    fn mt_on_transfer(
        &mut self,
        sender_id: AccountId,
        previous_owner_ids: Vec<AccountId>,
        token_ids: Vec<TokenId>,
        amounts: Vec<U128>,
        msg: String,
    ) -> PromiseOrValue<Vec<U128>> {
        for ((token_id, U128(amount)), previous_owner_id) in token_ids
            .iter()
            .zip(amounts.iter())
            .zip(previous_owner_ids.iter())
        {
            log!(
                "Received {} of {} from {} via {}",
                amount,
                token_id,
                previous_owner_id,
                sender_id,
            );
        }

        if msg == "panic" {
            env::panic_str("panic requested");
        } else if let Some(account_id) = msg.strip_prefix("transfer:") {
            let account_id: AccountId = account_id.parse().unwrap();

            log!("Transferring all to {}", account_id);

            return ext_nep245::ext(env::predecessor_account_id())
                .with_attached_deposit(NearToken::from_yoctonear(1u128))
                .mt_batch_transfer(account_id, token_ids, amounts.clone(), None, None)
                .then(Contract::ext(env::current_account_id()).return_value(amounts)) // ask to return the token even though we don't own it anymore
                .into();
        }

        PromiseOrValue::Value(if msg == "return" {
            amounts
        } else if let Some(return_amounts) = msg.strip_prefix("return:") {
            let return_amounts: Vec<U128> = serde_json::from_str(return_amounts).unwrap();
            return_amounts
        } else {
            vec![U128(0); amounts.len()]
        })
    }
}
