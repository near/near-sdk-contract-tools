//! Utility functions for storage key generation, storage fee management

use near_sdk::{env, require, Promise};

/// Concatenate bytes to form a key. Useful for generating storage keys.
///
/// # Examples
///
/// ```
/// use near_sdk_contract_tools::utils::prefix_key;
///
/// assert_eq!(prefix_key(b"p", b"key"), b"pkey");
/// ```
pub fn prefix_key(prefix: &[u8], key: &[u8]) -> Vec<u8> {
    [prefix, key].concat()
}

/// Calculates the storage fee of an action, given an initial storage amount,
/// and refunds the predecessor a portion of the attached deposit if necessary.
/// Returns refund Promise if refund was applied.
///
/// # Warning
///
/// New collections (those in `near_sdk::store`) cache writes, only applying
/// state changes on drop. However, this function only accounts for actual
/// changes to storage usage. You can force writes (allowing this function to
/// detect storage changes) by calling `.flush()` on `near_sdk::store::*`
/// collections.
///
/// # Examples
///
/// ```
/// use near_sdk_contract_tools::utils::apply_storage_fee_and_refund;
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

    let total_required_deposit = storage_fee
        .checked_add(additional_fees)
        .expect("Required deposit overflows u128");

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

/// Asserts that the attached deposit is greater than zero.
pub fn assert_nonzero_deposit() {
    require!(
        env::attached_deposit() > 0,
        "Attached deposit must be greater than zero"
    );
}

/// Generic hook trait for injecting code before and after component functions.
pub trait Hook<C, A = ()> {
    /// Before hook. Returns state to be passed to after hook.
    fn before(contract: &C, args: &A) -> Self;

    /// After hook. Receives state from before hook.
    fn after(contract: &mut C, args: &A, state: Self);
}

impl<C, A> Hook<C, A> for () {
    fn before(_contract: &C, _args: &A) {}
    fn after(_contract: &mut C, _args: &A, _: ()) {}
}

impl<C, A, T, U> Hook<C, A> for (T, U)
where
    T: Hook<C, A>,
    U: Hook<C, A>,
{
    fn before(contract: &C, args: &A) -> Self {
        (T::before(contract, args), U::before(contract, args))
    }

    fn after(contract: &mut C, args: &A, (t_state, u_state): Self) {
        T::after(contract, args, t_state);
        U::after(contract, args, u_state);
    }
}

#[cfg(test)]
mod tests {
    use super::prefix_key;

    #[test]
    fn test_prefix_key() {
        assert_eq!(prefix_key(b"a", b"b"), b"ab");
        assert_eq!(prefix_key("a".as_ref(), "b".as_ref()), b"ab");
        assert_eq!(prefix_key("a".as_ref(), b"b"), b"ab");
        assert_eq!(prefix_key(&[], "abc".as_ref()), b"abc");
        assert_eq!(prefix_key(&[], b""), [0u8; 0]);
        assert_eq!(prefix_key("abc".as_ref(), b""), b"abc");
    }
}
