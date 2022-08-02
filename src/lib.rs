//! Helpful functions and macros for developing smart contracts on NEAR Protocol.

#![warn(missing_docs)]

pub mod standard;

pub mod event;
pub mod migrate;
pub mod owner;
pub mod pause;
pub mod rbac;
pub mod slot;
pub mod utils;

pub use near_contract_tools_macros::*;

mod near_contract_tools {
    pub use super::*;
}
