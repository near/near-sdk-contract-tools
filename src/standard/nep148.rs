//! NEP-148 fungible token metadata implementation
//! <https://github.com/near/NEPs/blob/master/neps/nep-0148.md>
#![allow(missing_docs)] // ext_contract doesn't play nice with #![warn(missing_docs)]

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
#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize, Clone, PartialEq, Debug)]
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

#[cfg(test)]
mod tests {
    use crate::standard::nep148::FungibleTokenMetadata;
    use near_sdk::borsh::BorshSerialize;
    use std::borrow::Cow;

    #[test]
    fn borsh_serialization_ignores_cow() {
        let m1 = FungibleTokenMetadata {
            spec: "spec".into(),
            name: "name".into(),
            symbol: "symbol".into(),
            icon: Some("icon".into()),
            reference: Some("reference".into()),
            reference_hash: Some(Cow::Owned(b"reference_hash".to_vec().into())),
            decimals: 18,
        };

        let m2 = FungibleTokenMetadata {
            spec: "spec".to_owned().into(),
            name: "name".to_owned().into(),
            symbol: "symbol".to_owned().into(),
            icon: Some("icon".to_owned().into()),
            reference: Some("reference".to_owned().into()),
            reference_hash: Some(Cow::Owned(b"reference_hash".to_vec().into())),
            decimals: 18,
        };

        assert!(matches!(m1.spec, Cow::Borrowed(_)));
        assert!(matches!(m2.spec, Cow::Owned(_)));

        assert!(matches!(m1.name, Cow::Borrowed(_)));
        assert!(matches!(m2.name, Cow::Owned(_)));

        assert!(matches!(m1.symbol, Cow::Borrowed(_)));
        assert!(matches!(m2.symbol, Cow::Owned(_)));

        assert!(matches!(m1.icon, Some(Cow::Borrowed(_))));
        assert!(matches!(m2.icon, Some(Cow::Owned(_))));

        assert!(matches!(m1.reference, Some(Cow::Borrowed(_))));
        assert!(matches!(m2.reference, Some(Cow::Owned(_))));

        assert_eq!(m1, m2);

        let m1_serialized = m1.try_to_vec().unwrap();
        let m2_serialized = m2.try_to_vec().unwrap();

        assert_eq!(m1_serialized, m2_serialized);
    }
}
