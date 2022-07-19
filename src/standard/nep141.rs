use near_sdk::{
    borsh::{self, BorshSerialize},
    env, ext_contract,
    json_types::U128,
    require, AccountId, BorshStorageKey, Promise, PromiseOrValue,
};

use crate::slot::Slot;

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    TotalSupply,
    Account(AccountId),
}

pub trait Nep141Controller {
    fn root(&self) -> Slot<()>;

    fn slot_account(&self, account_id: &AccountId) -> Slot<u128> {
        self.root().field(StorageKey::Account(account_id.clone()))
    }

    fn slot_total_supply(&self) -> Slot<u128> {
        self.root().field(StorageKey::TotalSupply)
    }

    fn balance_of(&self, account_id: &AccountId) -> u128 {
        self.slot_account(account_id).read().unwrap_or(0)
    }

    fn total_supply(&self) -> u128 {
        self.slot_total_supply().read().unwrap_or(0)
    }

    fn withdraw(&mut self, account_id: &AccountId, amount: u128) {
        if amount != 0 {
            let balance = self.balance_of(account_id);
            if let Some(balance) = balance.checked_sub(amount) {
                self.slot_account(account_id).write(&balance);
            } else {
                env::panic_str("Balance underflow");
            }

            let total_supply = self.total_supply();
            if let Some(total_supply) = total_supply.checked_sub(amount) {
                self.slot_total_supply().write(&total_supply);
            } else {
                env::panic_str("Total supply underflow");
            }
        }
    }

    fn deposit(&mut self, account_id: &AccountId, amount: u128) {
        if amount != 0 {
            let balance = self.balance_of(account_id);
            if let Some(balance) = balance.checked_add(amount) {
                self.slot_account(account_id).write(&balance);
            } else {
                env::panic_str("Balance overflow");
            }

            let total_supply = self.total_supply();
            if let Some(total_supply) = total_supply.checked_add(amount) {
                self.slot_total_supply().write(&total_supply);
            } else {
                env::panic_str("Total supply overflow");
            }
        }
    }

    fn transfer(
        &mut self,
        sender_account_id: &AccountId,
        receiver_account_id: &AccountId,
        amount: u128,
    ) {
        let sender_balance = self.balance_of(sender_account_id);

        if let Some(sender_balance) = sender_balance.checked_sub(amount) {
            let receiver_balance = self.balance_of(receiver_account_id);
            if let Some(receiver_balance) = receiver_balance.checked_add(amount) {
                self.slot_account(sender_account_id).write(&sender_balance);
                self.slot_account(receiver_account_id)
                    .write(&receiver_balance);
            } else {
                env::panic_str("Receiver balance overflow");
            }
        } else {
            env::panic_str("Sender balance underflow");
        }
    }
}

#[ext_contract(ext_nep141_receiver)]
pub trait Nep141Receiver {
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128>;
}

#[ext_contract(ext_nep141)]
pub trait Nep141External {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> Promise;
    fn ft_total_supply(&self) -> U128;
    fn ft_balance_of(&self, account_id: AccountId) -> U128;
}
