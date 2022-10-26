//! Utility functions for storage key generation, storage fee management

use near_sdk::{env, require, Promise};

/// Concatenate bytes to form a key. Useful for generating storage keys.
///
/// # Examples
///
/// ```
/// use near_contract_tools::utils::prefix_key;
///
/// assert_eq!(prefix_key(b"p", b"key"), b"pkey".to_vec());
/// ```
pub fn prefix_key(prefix: &dyn AsRef<[u8]>, key: &dyn AsRef<[u8]>) -> Vec<u8> {
    [prefix.as_ref(), key.as_ref()].concat()
}

/// Calculate the storage fee of an action, given an initial storage amount,
/// and refund the predecessor a portion of the attached deposit if necessary.
/// Return a refund Promise if refund was applied.
///
/// # Examples
///
/// ```
/// use near_contract_tools::utils::apply_storage_fee_and_refund;
///
/// let initial_storage_usage = near_sdk::env::storage_usage();
/// let additional_fees = 0;
///
/// // Action that consumes storage.
/// near_sdk::env::storage_write(b"key", b"value");
///
/// near_sdk::testing_env!(near_sdk::test_utils::VMContextBuilder::new()
///     .attached_deposit(near_sdk::ONE_NEAR)
///     .build());
/// // Attached deposit must cover storage fee or this function will panic
/// apply_storage_fee_and_refund(initial_storage_usage, additional_fees);
/// ```
pub fn apply_storage_fee_and_refund(
    initial_storage_usage: u64,
    additional_fees: u128,
) -> Option<Promise> {
    // Storage consumption after storage event
    let storage_usage_end = env::storage_usage();

    // Storage fee incurred by storage event, clamped >= 0
    let storage_fee = u128::from(storage_usage_end.saturating_sub(initial_storage_usage))
        * env::storage_byte_cost();

    let total_required_deposit = storage_fee + additional_fees;

    let attached_deposit = env::attached_deposit();

    require!(
        attached_deposit >= total_required_deposit,
        format!(
            "Insufficient deposit: attached {attached_deposit} yoctoNEAR < required {total_required_deposit} yoctoNEAR ({storage_fee} storage + {additional_fees} additional)",
        )
    );

    let refund = attached_deposit - total_required_deposit;

    // Send refund transfer if required
    if refund > 0 {
        Some(Promise::new(env::predecessor_account_id()).transfer(refund))
    } else {
        None
    }
}
