use std::borrow::Cow;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    json_types::Base64VecU8, ext_contract,
};
use serde::{Deserialize, Serialize};

pub const FT_METADATA_SPEC: &str = "ft-1.0.0";

#[derive(BorshDeserialize, BorshSerialize, Clone, Deserialize, Serialize, Debug)]
pub struct FungibleTokenMetadata<'a> {
    pub spec: Cow<'a, str>,
    pub name: Cow<'a, str>,
    pub symbol: Cow<'a, str>,
    pub icon: Option<Cow<'a, str>>,
    pub reference: Option<Cow<'a, str>>,
    pub reference_hash: Option<Cow<'a, Base64VecU8>>,
    pub decimals: u8,
}

#[ext_contract(ext_nep148)]
pub trait Nep148 {
    fn ft_metadata(&self) -> FungibleTokenMetadata<'static>;
}
