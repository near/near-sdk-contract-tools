use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env,
    json_types::U128,
    log, near_bindgen,
    store::LookupMap,
    AccountId, PanicOnDefault,
};
use near_sdk_contract_tools::{standard::nep145::*, Nep145};

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Nep145)]
#[near_bindgen]
pub struct Contract {
    pub storage: LookupMap<AccountId, Vec<u64>>,
}

impl Nep145Hook for Contract {
    fn after_force_unregister(
        _contract: &mut Self,
        _account_id: &near_sdk::AccountId,
        _balance: &near_sdk_contract_tools::standard::nep145::StorageBalance,
    ) {
        log!("After force unregister");
    }
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self {
            storage: LookupMap::new(b"s"),
        };

        Nep145Controller::set_storage_balance_bounds(
            &mut contract,
            &StorageBalanceBounds {
                min: U128(0),
                max: None,
            },
        );

        contract
    }

    pub fn use_storage(&mut self, num: u64) {
        let storage_usage_start = env::storage_usage();

        let predecessor = env::predecessor_account_id();

        self.storage.insert(predecessor.clone(), (0..num).collect());

        self.storage.flush();

        let storage_usage = env::storage_usage() - storage_usage_start;
        let storage_fee = env::storage_byte_cost() * storage_usage as u128;

        Nep145Controller::storage_lock(self, &predecessor, storage_fee.into())
            .unwrap_or_else(|e| env::panic_str(&format!("Storage lock error: {}", e)));
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::{test_utils::VMContextBuilder, testing_env, ONE_NEAR};

    use super::*;

    fn alice() -> AccountId {
        "alice.near".parse().unwrap()
    }

    #[test]
    fn storage_sanity_check() {
        let byte_cost = env::storage_byte_cost();

        let mut contract = Contract::new();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(alice())
            .attached_deposit(ONE_NEAR)
            .build());

        Nep145::storage_deposit(&mut contract, None, None);

        assert_eq!(
            Nep145::storage_balance_of(&contract, alice()),
            Some(StorageBalance {
                total: U128(ONE_NEAR),
                available: U128(ONE_NEAR),
            }),
        );

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(alice())
            .build());

        contract.use_storage(1000);

        let first = Nep145::storage_balance_of(&contract, alice()).unwrap();

        assert_eq!(first.total.0, ONE_NEAR);
        assert!(ONE_NEAR - (first.available.0 + 8 * 1000 * byte_cost) < 100 * byte_cost); // about 100 bytes for storing keys, etc.

        contract.use_storage(2000);

        let second = Nep145::storage_balance_of(&contract, alice()).unwrap();

        assert_eq!(second.total.0, ONE_NEAR);
        assert_eq!(second.available.0, first.available.0 - 8 * 1000 * byte_cost);
    }
}
