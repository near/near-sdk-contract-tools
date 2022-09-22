#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

pub mod standard;

pub mod approval;
pub mod migrate;
pub mod owner;
pub mod pause;
pub mod rbac;
pub mod slot;
pub mod utils;

pub use near_contract_tools_macros::*;
