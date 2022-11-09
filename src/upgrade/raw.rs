//! Accepts contract deployment as raw binary.
//!
//! This pattern was common in NEAR smart contracts at the time of writing,
//! but is included here mostly for compatibility/legacy reasons. Unless you
//! are really sure you know what you are doing, you should probably be using
//! [`super::serialized`].
//!
//! # Warning
//!
//! Functions in this module are generally _not callable_ from any call tree
//! originating from a function annotated by `#[near_bindgen]`.

use std::marker::PhantomData;

use near_sdk::{env, sys};

use super::PostUpgrade;

/// This function performs low-level, `unsafe` interactions with the NEAR VM.
/// This function automatically sets the return value of the function call to
/// the contract deployment &rarr; migrate function call promise, so the
/// contract should not try to return any other values. This also means that
/// this function probably should not be called from a `#[near_bindgen]`
/// context, since the macro may automatically set a different return value.
///
/// # Safety
///
/// If the `source` is `RawUpgradeSource::FatPointer`, the `length` and
/// `pointer` fields must be valid values to pass into
/// `near_sys::promise_batch_action_deploy_contract` (i.e. pointer to a valid
/// WASM blob or a register descriptor).
pub fn upgrade(source: RawUpgradeSource, post_upgrade: Option<PostUpgrade>) {
    unsafe {
        // Create a promise batch
        let promise_id = sys::promise_batch_create(
            env::current_account_id().as_bytes().len() as u64,
            env::current_account_id().as_bytes().as_ptr() as u64,
        );

        let (len, ptr) = match source {
            RawUpgradeSource::TransactionInput => {
                sys::input(0);
                (u64::MAX, 0)
            }
            RawUpgradeSource::FatPointer {
                length, pointer, ..
            } => (length, pointer),
        };

        // Deploy the contract code
        sys::promise_batch_action_deploy_contract(promise_id, len, ptr);

        if let Some(post_upgrade) = post_upgrade {
            // Call promise to migrate the state.
            // Batched together to fail upgrade if migration fails.
            sys::promise_batch_action_function_call_weight(
                promise_id,
                post_upgrade.method.len() as u64,
                post_upgrade.method.as_ptr() as u64,
                post_upgrade.args.len() as u64,
                post_upgrade.args.as_ptr() as u64,
                0,
                post_upgrade.minimum_gas.0,
                u64::MAX,
            );
        }

        sys::promise_return(promise_id);
    }
}

/// Where can the [`upgrade`] function find the code to deploy?
#[derive(Debug, Clone)]
pub enum RawUpgradeSource<'a> {
    /// Use the input to the transaction directly as binary WASM.
    TransactionInput,
    /// Use a binary pointer from elsewhere in the program.
    FatPointer {
        /// Data length
        length: u64,
        /// Pointer location
        pointer: u64,
        /// If the pointer is derived from a volatile value, this constrains
        /// its lifetime. Otherwise, if the pointer is derived from `'static`
        /// data or is a NEAR VM register descriptor, this can be `'static`
        /// (i.e. ignored).
        _lifetime: PhantomData<&'a ()>,
    },
}

impl<'a> Default for RawUpgradeSource<'a> {
    fn default() -> Self {
        RawUpgradeSource::TransactionInput
    }
}

impl<'a> From<&'a Vec<u8>> for RawUpgradeSource<'a> {
    fn from(v: &'a Vec<u8>) -> Self {
        Self::FatPointer {
            length: v.len() as u64,
            pointer: v.as_ptr() as u64,
            _lifetime: PhantomData,
        }
    }
}
