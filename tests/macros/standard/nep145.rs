use near_sdk::{AccountId, NearToken, PanicOnDefault, env, log, near, store::LookupMap};
use near_sdk_contract_tools::{Nep145, hook::Hook, standard::nep145::*};

#[derive(Nep145, PanicOnDefault)]
#[nep145(force_unregister_hook = "ForceUnregisterHook")]
#[near(contract_state)]
pub struct Contract {
    pub storage: LookupMap<AccountId, Vec<u64>>,
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

#[near]
impl Contract {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self {
            storage: LookupMap::new(b"s"),
        };

        Nep145Controller::set_storage_balance_bounds(
            &mut contract,
            &StorageBalanceBounds {
                min: NearToken::from_yoctonear(0),
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

        Nep145Controller::lock_storage(self, &predecessor, storage_fee)
            .unwrap_or_else(|e| env::panic_str(&format!("Storage lock error: {e}")));
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::{NearToken, test_utils::VMContextBuilder, testing_env};

    use super::*;

    fn alice() -> AccountId {
        "alice.near".parse().unwrap()
    }

    #[test]
    fn storage_sanity_check() {
        let one_near = NearToken::from_near(1u128);
        let byte_cost = env::storage_byte_cost();

        let mut contract = Contract::new();

        testing_env!(
            VMContextBuilder::new()
                .predecessor_account_id(alice())
                .attached_deposit(one_near)
                .build()
        );

        Nep145::storage_deposit(&mut contract, None, None);

        assert_eq!(
            Nep145::storage_balance_of(&contract, alice()),
            Some(StorageBalance {
                total: one_near,
                available: one_near,
            }),
        );

        testing_env!(
            VMContextBuilder::new()
                .predecessor_account_id(alice())
                .build()
        );

        contract.use_storage(1000);

        let first = Nep145::storage_balance_of(&contract, alice()).unwrap();

        assert_eq!(first.total, one_near);
        assert!(
            one_near.as_yoctonear()
                - (first.available.as_yoctonear() + 8 * 1000 * byte_cost.as_yoctonear())
                < 100 * byte_cost.as_yoctonear()
        ); // about 100 bytes for storing keys, etc.

        contract.use_storage(2000);

        let second = Nep145::storage_balance_of(&contract, alice()).unwrap();

        assert_eq!(second.total, one_near);
        assert_eq!(
            second.available.as_yoctonear(),
            first.available.as_yoctonear() - 8 * 1000 * byte_cost.as_yoctonear()
        );

        let available = second.available;
        let half_available = available.saturating_div(2);

        testing_env!(
            VMContextBuilder::new()
                .predecessor_account_id(alice())
                .attached_deposit(NearToken::from_yoctonear(1))
                .build()
        );

        Nep145::storage_withdraw(&mut contract, Some(half_available));

        let third = Nep145::storage_balance_of(&contract, alice()).unwrap();

        assert_eq!(third.total, one_near.saturating_sub(half_available));
        assert_eq!(third.available, half_available);

        Nep145::storage_withdraw(&mut contract, None);

        let fourth = Nep145::storage_balance_of(&contract, alice()).unwrap();

        assert_eq!(fourth.total, one_near.saturating_sub(available));
        assert_eq!(fourth.available, NearToken::from_yoctonear(0));
    }

    #[test]
    #[should_panic = "insufficient balance"]
    fn storage_over_lock_fail() {
        let one_near = NearToken::from_near(1u128);
        let byte_cost = env::storage_byte_cost();

        let mut contract = Contract::new();

        testing_env!(
            VMContextBuilder::new()
                .predecessor_account_id(alice())
                .attached_deposit(one_near)
                .build()
        );

        Nep145::storage_deposit(&mut contract, None, None);

        testing_env!(
            VMContextBuilder::new()
                .predecessor_account_id(alice())
                .build()
        );

        #[allow(clippy::cast_possible_truncation)]
        contract.use_storage(
            one_near
                .as_yoctonear()
                .saturating_div(byte_cost.as_yoctonear()) as u64
                + 1,
        );
    }

    #[test]
    #[should_panic = "insufficient balance"]
    fn storage_over_withdraw_fail() {
        let mut contract = Contract::new();

        testing_env!(
            VMContextBuilder::new()
                .predecessor_account_id(alice())
                .attached_deposit(NearToken::from_near(1))
                .build()
        );

        Nep145::storage_deposit(&mut contract, None, None);

        let balance = Nep145::storage_balance_of(&contract, alice()).unwrap();

        testing_env!(
            VMContextBuilder::new()
                .predecessor_account_id(alice())
                .attached_deposit(NearToken::from_yoctonear(1))
                .build()
        );

        Nep145::storage_withdraw(
            &mut contract,
            Some(
                balance
                    .available
                    .saturating_add(NearToken::from_yoctonear(1)),
            ),
        );
    }
}
