//! King of the Hill Game
//!
//! A simple blockchain game for NEAR Protocol inspired by [King of the Ether](https://www.kingoftheether.com/thrones/kingoftheether/index.html#WhatItDo).
#![allow(missing_docs)]

pub fn main() {} // Ignore

use near_contract_tools::{
    event, owner::*, rbac::*, standard::nep297::Event, upgrade::serialized::UpgradeHook, Owner,
    Rbac, Upgrade,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::{U128, U64},
    near_bindgen, require,
    serde::Serialize,
    store::{LookupMap, Vector},
    AccountId, BorshStorageKey, PanicOnDefault, Promise,
};

#[event(standard = "x-king", version = "1.0.0")]
pub enum KingEvent {
    NewMonarch {
        old_monarch: Option<AccountId>,
        new_monarch: AccountId,
    },
}

#[derive(BorshSerialize, BorshStorageKey)]
pub enum Role {
    Royal,
    KingdomResident,
}

#[derive(BorshSerialize, BorshStorageKey)]
pub enum StorageKey {
    History,
    Credit,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub struct Monarch {
    pub account_id: AccountId,
    pub coronation_timestamp_milliseconds: U64,
    pub claim_price_paid: U128,
}

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Owner, Rbac, Upgrade)]
#[rbac(roles = "Role")]
#[near_bindgen]
pub struct KingGame {
    pub kingdom_name: String,
    pub resident_tax: U128,
    pub starting_claim_price: U128,
    pub maximum_claim_price: U128,
    pub monarch_lifespan_milliseconds: U64,
    pub increase_claim_price_by_thousanths: u16,
    pub owner_commission_thousanths: u16,

    pub history: Vector<Monarch>,
    pub credit: LookupMap<AccountId, U128>,
}

#[near_bindgen]
impl KingGame {
    #[init]
    pub fn new(
        kingdom_name: String,
        resident_tax: U128,
        starting_claim_price: U128,
        maximum_claim_price: U128,
        monarch_lifespan_milliseconds: U64,
        increase_claim_price_by_thousanths: u16,
        owner_commission_thousanths: u16,
    ) -> Self {
        let mut contract = Self {
            kingdom_name,
            resident_tax,
            starting_claim_price,
            maximum_claim_price,
            monarch_lifespan_milliseconds,
            increase_claim_price_by_thousanths,
            owner_commission_thousanths,

            history: Vector::new(StorageKey::History),
            credit: LookupMap::new(StorageKey::Credit),
        };

        Owner::init(&mut contract, &env::predecessor_account_id());

        contract
    }

    pub fn is_current_monarch_alive(&self) -> bool {
        self.get_current_living_monarch().is_some()
    }

    pub fn get_current_living_monarch(&self) -> Option<&Monarch> {
        self.get_latest_monarch().filter(|m| {
            env::block_timestamp_ms() - m.coronation_timestamp_milliseconds.0
                < self.monarch_lifespan_milliseconds.0
        })
    }

    pub fn get_latest_monarch(&self) -> Option<&Monarch> {
        self.history.get(self.history.len() - 1)
    }

    pub fn get_claim_price(&self) -> U128 {
        self.get_current_living_monarch()
            .map(|m| {
                let next = m.claim_price_paid.0
                    + m.claim_price_paid.0 * self.increase_claim_price_by_thousanths as u128 / 1000;

                u128::min(next, self.maximum_claim_price.0).into()
            })
            .unwrap_or_else(|| self.starting_claim_price)
    }

    pub fn get_tax(&self) -> U128 {
        self.resident_tax
    }

    #[payable]
    pub fn become_resident(&mut self) {
        Self::prohibit_role(&Role::KingdomResident);
        let predecessor = env::predecessor_account_id();

        require!(
            env::attached_deposit() >= self.resident_tax.0,
            "Insufficient tax!",
        );

        self.add_role(predecessor.clone(), &Role::KingdomResident);
    }

    fn credit_account(&mut self, account_id: AccountId, amount: u128) {
        let current_credit: u128 = self
            .credit
            .get(&account_id)
            .map(|v| (*v).into())
            .unwrap_or(0);
        self.credit.set(
            account_id,
            Some(
                (current_credit
                    .checked_add(amount)
                    .expect("Account balance overflow"))
                .into(),
            ),
        );
    }

    fn debit_account(&mut self, account_id: AccountId, amount: u128) {
        let current_credit: u128 = self
            .credit
            .get(&account_id)
            .map(|v| (*v).into())
            .unwrap_or(0);
        self.credit.set(
            account_id,
            Some(
                (current_credit
                    .checked_sub(amount)
                    .expect("Account balance underflow"))
                .into(),
            ),
        );
    }

    pub fn get_credit(&self, account_id: AccountId) -> U128 {
        self.credit
            .get(&account_id)
            .map(|u| (*u))
            .unwrap_or(U128(0))
    }

    pub fn withdraw_credit(&mut self, amount: Option<U128>) -> Promise {
        let predecessor = env::predecessor_account_id();
        let amount: u128 = amount.map(Into::into).unwrap_or_else(|| {
            self.credit
                .get(&predecessor)
                .map(|u| (*u).into())
                .unwrap_or(0)
        });

        self.debit_account(predecessor.clone(), amount);

        Promise::new(predecessor).transfer(amount)
    }

    #[payable]
    pub fn claim(&mut self) {
        Self::require_role(&Role::KingdomResident);
        let claim_price = self.get_claim_price();
        let deposit = env::attached_deposit();
        require!(deposit >= claim_price.0, "Insufficient deposit!");

        // Distribute credits
        let excess = deposit - claim_price.0;
        let predecessor = env::predecessor_account_id();

        // Credit back overpayment
        if excess > 0 {
            self.credit_account(predecessor.clone(), excess);
        }

        // Credit the current monarch
        let usurped_account_id = self
            .get_current_living_monarch()
            .map(|u| u.account_id.clone());
        // If the current monarch exists/is alive, to how much of the claim
        // price are they entitled? The remainder gets credited to the owner
        // of the contract.
        let credit_to_owner = if let Some(ref usurped_account_id) = usurped_account_id {
            let owner_fee = claim_price.0 * self.owner_commission_thousanths as u128 / 1000;
            self.credit_account(usurped_account_id.clone(), claim_price.0 - owner_fee);
            owner_fee
        } else {
            claim_price.0
        };

        // Credit the remainder to the owner, if the owner exists.
        if let Some(owner_account_id) = self.own_get_owner() {
            self.credit_account(owner_account_id, credit_to_owner);
        }

        self.add_role(predecessor.clone(), &Role::Royal);

        let monarch = Monarch {
            account_id: predecessor.clone(),
            claim_price_paid: claim_price,
            coronation_timestamp_milliseconds: env::block_timestamp_ms().into(),
        };

        self.history.push(monarch);

        KingEvent::NewMonarch {
            old_monarch: usurped_account_id,
            new_monarch: predecessor,
        }
        .emit();
    }
}

impl UpgradeHook for KingGame {
    fn on_upgrade(&self) {
        Self::require_owner();
    }
}
