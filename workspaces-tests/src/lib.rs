#![allow(missing_docs)]
#![cfg(not(windows))]

#[macro_export]
macro_rules! predicate {
    () => {
        workspaces_tests::near_sdk!();
        near_sdk_contract_tools::compat_use_borsh!();

        pub fn main() {}
    };
}

#[macro_export]
macro_rules! near_sdk {
    () => {
        #[cfg(feature = "near-sdk-4")]
        extern crate near_sdk_4 as near_sdk;

        #[cfg(feature = "near-sdk-5")]
        extern crate near_sdk_5 as near_sdk;
    };
}
