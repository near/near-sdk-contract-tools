// TODO: Link to the standard
//! NEP-148 fungible token metadata implementation

use std::borrow::Cow;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    ext_contract,
    json_types::Base64VecU8,
};
use serde::{Deserialize, Serialize};

/// Version of the NEP-148 metadata spec
pub const FT_METADATA_SPEC: &str = "ft-1.0.0";

/// NEP-148-compatible metadata struct
#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize, Debug)]
pub struct FungibleTokenMetadata<'a> {
    /// Version of the NEP-148 spec
    pub spec: Cow<'a, str>,
    /// Human-friendly name of the token contract
    pub name: Cow<'a, str>,
    /// Short, ideally unique string to concisely identify the token contract
    pub symbol: Cow<'a, str>,
    /// String representation (HTTP URL, data URL, IPFS, Arweave, etc.) of an
    /// icon for this token
    pub icon: Option<Cow<'a, str>>,
    /// External (off-chain) URL to additional JSON metadata for this token contract
    pub reference: Option<Cow<'a, str>>,
    /// Hash of the content that should be present in the `reference` field.
    /// For tamper protection.
    pub reference_hash: Option<Cow<'a, Base64VecU8>>,
    /// Cosmetic. Number of base-10 decimal places to shift the floating point.
    /// 18 is a common value.
    pub decimals: u8,
}

/// Contract that supports the NEP-148 metadata standard
#[ext_contract(ext_nep148)]
pub trait Nep148 {
    /// Returns the metadata struct for this contract.
    fn ft_metadata(&self) -> FungibleTokenMetadata<'static>;
}
