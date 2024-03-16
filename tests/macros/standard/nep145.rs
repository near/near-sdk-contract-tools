compat_use_borsh!();
use near_sdk::{
    env, json_types::U128, log, near_bindgen, store::LookupMap, AccountId, PanicOnDefault,
};
use near_sdk_contract_tools::{
    compat_derive_borsh, compat_near_to_u128, compat_use_borsh, hook::Hook, standard::nep145::*,
    Nep145,
};

compat_derive_borsh! {
    #[derive(PanicOnDefault, Nep145)]
    #[nep145(force_unregister_hook = "ForceUnregisterHook")]
    #[near_bindgen]
    pub struct Contract {
        pub storage: LookupMap<AccountId, Vec<u64>>,
    }
}

pub struct ForceUnregisterHook;

impl Hook<Contract, Nep145ForceUnregister<'_>> for ForceUnregisterHook {
    fn hook<R>(
        contract: &mut Contract,
        _args: &Nep145ForceUnregister<'_>,
        f: impl FnOnce(&mut Contract) -> R,
    ) -> R {
        log!("Before force unregister");
        let r = f(contract);
        log!("After force unregister");
        r
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
        let storage_fee = env::storage_byte_cost().saturating_mul(u128::from(storage_usage));

        Nep145Controller::lock_storage(
            self,
            &predecessor,
            compat_near_to_u128!(storage_fee).into(),
        )
        .unwrap_or_else(|e| env::panic_str(&format!("Storage lock error: {}", e)));
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::{test_utils::VMContextBuilder, testing_env};
    use near_sdk_contract_tools::compat_near;

    use super::*;

    fn alice() -> AccountId {
        "alice.near".parse().unwrap()
    }

    #[test]
    fn storage_sanity_check() {
        let one_near = compat_near!(1u128);
        let one_near_u128 = compat_near_to_u128!(one_near);

        let byte_cost = compat_near_to_u128!(env::storage_byte_cost());

        let mut contract = Contract::new();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(alice())
            .attached_deposit(one_near)
            .build());

        Nep145::storage_deposit(&mut contract, None, None);

        assert_eq!(
            Nep145::storage_balance_of(&contract, alice()),
            Some(StorageBalance {
                total: U128(one_near_u128),
                available: U128(one_near_u128),
            }),
        );

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(alice())
            .build());

        contract.use_storage(1000);

        let first = Nep145::storage_balance_of(&contract, alice()).unwrap();

        assert_eq!(first.total.0, one_near_u128);
        assert!(one_near_u128 - (first.available.0 + 8 * 1000 * byte_cost) < 100 * byte_cost); // about 100 bytes for storing keys, etc.

        contract.use_storage(2000);

        let second = Nep145::storage_balance_of(&contract, alice()).unwrap();

        assert_eq!(second.total.0, one_near_u128);
        assert_eq!(second.available.0, first.available.0 - 8 * 1000 * byte_cost);
    }
}
