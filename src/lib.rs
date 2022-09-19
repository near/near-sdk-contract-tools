//! Helpful functions and macros for developing smart contracts on NEAR Protocol.
#![doc = include_str!("../README.md")]

pub mod standard;

pub mod approval;
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

#[cfg(doctest)]
mod README_md {
    doc_comment::doctest!("../README.md");
}
