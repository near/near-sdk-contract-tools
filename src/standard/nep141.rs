use near_sdk::{
    assert_one_yocto,
    borsh::{self, BorshSerialize},
    env, ext_contract,
    json_types::U128,
    require, AccountId, BorshStorageKey, Gas, Promise, PromiseOrValue, PromiseResult,
};
use serde::Serialize;

use crate::{event::Event, slot::Slot};
use crate::{near_contract_tools, Event};

pub const GAS_FOR_RESOLVE_TRANSFER: Gas = Gas(5_000_000_000_000);
pub const GAS_FOR_FT_TRANSFER_CALL: Gas = Gas(25_000_000_000_000 + GAS_FOR_RESOLVE_TRANSFER.0);

#[derive(Serialize, Event)]
#[event(standard = "nep141", version = "1.0.0", rename_all = "snake_case")]
#[serde(untagged)]
pub enum Nep141Event<'a> {
    FtMint {
        owner_id: &'a AccountId,
        amount: &'a U128,
        #[serde(skip_serializing_if = "Option::is_none")]
        memo: Option<&'a str>,
    },
    FtTransfer {
        old_owner_id: &'a AccountId,
        new_owner_id: &'a AccountId,
        amount: &'a U128,
        #[serde(skip_serializing_if = "Option::is_none")]
        memo: Option<&'a str>,
    },
    FtBurn {
        owner_id: &'a AccountId,
        amount: &'a U128,
        #[serde(skip_serializing_if = "Option::is_none")]
        memo: Option<&'a str>,
    },
}

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

    fn internal_withdraw(&mut self, account_id: &AccountId, amount: u128) {
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

    fn internal_deposit(&mut self, account_id: &AccountId, amount: u128) {
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

    fn internal_transfer(
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

    fn transfer(
        &mut self,
        sender_account_id: &AccountId,
        receiver_account_id: &AccountId,
        amount: u128,
        memo: Option<&str>,
    ) {
        self.internal_transfer(sender_account_id, receiver_account_id, amount);

        Nep141Event::FtTransfer {
            old_owner_id: sender_account_id,
            new_owner_id: receiver_account_id,
            amount: &amount.into(),
            memo,
        }
        .emit();
    }

    fn mint(&mut self, account_id: &AccountId, amount: u128, memo: Option<&str>) {
        self.internal_deposit(account_id, amount);

        Nep141Event::FtMint {
            owner_id: account_id,
            amount: &amount.into(),
            memo,
        }
        .emit();
    }

    fn burn(&mut self, account_id: &AccountId, amount: u128, memo: Option<&str>) {
        self.internal_withdraw(account_id, amount);

        Nep141Event::FtBurn {
            owner_id: account_id,
            amount: &amount.into(),
            memo,
        }
        .emit();
    }

    fn transfer_call(
        &mut self,
        sender_account_id: AccountId,
        receiver_account_id: AccountId,
        amount: u128,
        memo: Option<&str>,
        msg: String,
        gas_allowance: Gas,
    ) -> Promise {
        require!(
            gas_allowance > GAS_FOR_FT_TRANSFER_CALL,
            "More gas is required",
        );

        self.transfer(&sender_account_id, &receiver_account_id, amount, memo);

        let receiver_gas = gas_allowance
            .0
            .checked_sub(GAS_FOR_FT_TRANSFER_CALL.0)
            .unwrap_or_else(|| env::panic_str("Prepaid gas overflow"));
        // Initiating receiver's call and the callback
        ext_nep141_receiver::ext(receiver_account_id.clone())
            .with_static_gas(receiver_gas.into())
            .ft_on_transfer(sender_account_id.clone(), amount.into(), msg)
            .then(
                ext_nep141_resolver::ext(env::current_account_id())
                    .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                    .ft_resolve_transfer(sender_account_id, receiver_account_id, amount.into()),
            )
    }

    fn resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: u128,
    ) -> u128 {
        let ft_on_transfer_promise_result = env::promise_result(0);

        let unused_amount = match ft_on_transfer_promise_result {
            PromiseResult::NotReady => env::abort(),
            PromiseResult::Successful(value) => {
                if let Ok(U128(unused_amount)) = serde_json::from_slice::<U128>(&value) {
                    std::cmp::min(amount, unused_amount)
                } else {
                    amount
                }
            }
            PromiseResult::Failed => amount,
        };

        let refunded_amount = if unused_amount > 0 {
            let receiver_balance = self.balance_of(&receiver_id);
            if receiver_balance > 0 {
                let refund_amount = std::cmp::min(receiver_balance, unused_amount);
                self.transfer(&receiver_id, &sender_id, refund_amount, None);
                refund_amount
            } else {
                0
            }
        } else {
            0
        };

        // Used amount
        amount - refunded_amount
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

#[ext_contract(ext_nep141_resolver)]
pub trait Nep141Resolver {
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128;
}

#[ext_contract(ext_nep141)]
pub trait Nep141 {
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
