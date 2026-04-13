//! Helper type for storing [`AccountId`]s in a constant number of bytes.

use std::{io::ErrorKind, ops::Deref, str::FromStr};

use near_sdk::{
    AccountId, AccountIdRef,
    borsh::{self, BorshDeserialize, BorshSerialize, io::Error},
    near,
};

/// Borsh-serializes a [`AccountId`] to a constant number of bytes to avoid
/// storage accounting edge cases.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[near(serializers = [json, borsh])]
#[serde(transparent)]
pub struct CSAccountId(
    #[borsh(serialize_with = "cs_ser", deserialize_with = "cs_de")] pub AccountId,
);

#[derive(BorshSerialize, BorshDeserialize, Debug)]
#[borsh(crate = "near_sdk::borsh")]
struct Container {
    len: u8,
    bytes: [u8; AccountId::MAX_LEN],
}

fn cs_ser(account_id: &AccountId, writer: &mut impl borsh::io::Write) -> Result<(), Error> {
    // Assert length will fit in u8
    const _: () = assert!(AccountId::MAX_LEN <= u8::MAX as usize);

    let mut buf = [0_u8; AccountId::MAX_LEN];
    let var_bytes = account_id.as_str().as_bytes();
    let len = var_bytes.len();
    buf[0..len].copy_from_slice(var_bytes);
    #[allow(
        clippy::cast_possible_truncation,
        reason = "guaranteed safe by const assertion"
    )]
    BorshSerialize::serialize(
        &Container {
            len: len as u8,
            bytes: buf,
        },
        writer,
    )?;
    Ok(())
}

fn invalid(e: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Error {
    Error::new(ErrorKind::InvalidData, e)
}

fn cs_de(reader: &mut impl borsh::io::Read) -> Result<AccountId, Error> {
    let de: Container = BorshDeserialize::deserialize_reader(reader)?;
    let de_str = std::str::from_utf8(&de.bytes[..de.len as usize]).map_err(invalid)?;
    AccountId::from_str(de_str).map_err(invalid)
}

impl Deref for CSAccountId {
    type Target = AccountIdRef;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<AccountId> for CSAccountId {
    fn from(value: AccountId) -> Self {
        Self(value)
    }
}

impl From<CSAccountId> for AccountId {
    fn from(value: CSAccountId) -> Self {
        value.0
    }
}

impl AsRef<AccountId> for CSAccountId {
    fn as_ref(&self) -> &AccountId {
        &self.0
    }
}

impl AsRef<AccountIdRef> for CSAccountId {
    fn as_ref(&self) -> &AccountIdRef {
        &self.0
    }
}

impl AsRef<str> for CSAccountId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ser_de() {
        let alice: AccountId = "alice".parse().unwrap();
        let long: AccountId = "0000000000000000000000000000000000000000000000000000000000000000"
            .parse()
            .unwrap();
        let c_alice = CSAccountId::from(alice.clone());
        let borsh_alice = borsh::to_vec(&c_alice).unwrap();
        let c_long = CSAccountId::from(long.clone());
        let borsh_long = borsh::to_vec(&c_long).unwrap();
        assert_eq!(borsh_alice.len(), borsh_long.len());
        let d_alice: CSAccountId = borsh::from_slice(&borsh_alice).unwrap();
        let d_long: CSAccountId = borsh::from_slice(&borsh_long).unwrap();
        assert_eq!(d_alice.0, alice);
        assert_eq!(d_long.0, long);
    }
}
