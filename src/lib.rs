//! Helpful functions and macros for developing smart contracts on NEAR Protocol.

#![warn(missing_docs)]

pub mod event;
pub mod ownership;
pub mod pausable;
pub mod rbac;
pub mod utils;

pub use near_contract_tools_macros::*;

mod near_contract_tools {
    pub use super::*;
}
