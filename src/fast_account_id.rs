//! A fast alternative to `near_sdk::AccountId` that is faster to use, and has a
//! smaller Borsh serialization footprint.

use std::{ops::Deref, sync::Arc};

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

/// An alternative to `near_sdk::AccountId` that is faster to use, and has a
/// smaller Borsh serialization footprint.
///
/// Limitations:
///  - Does not implement `serde` serialization traits.
///  - No parsing/validation logic. This is basically just a string wrapper
///     with NEAR-account-ID-specific serialization logic.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FastAccountId(Arc<str>);

impl std::fmt::Display for FastAccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl Deref for FastAccountId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for FastAccountId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<near_sdk::AccountId> for FastAccountId {
    fn from(account_id: near_sdk::AccountId) -> Self {
        Self(Arc::from(account_id.as_str()))
    }
}

impl From<&str> for FastAccountId {
    fn from(account_id: &str) -> Self {
        Self(Arc::from(account_id))
    }
}

impl BorshSerialize for FastAccountId {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let len: u8 = self.0.len() as u8;
        writer.write_all(&[len])?;
        let compressed = compress_account_id(&self.0).unwrap();
        writer.write_all(&compressed)?;
        Ok(())
    }
}

impl BorshDeserialize for FastAccountId {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let len = buf[0] as usize;
        let compressed = &buf[1..];
        let account_id = decompress_account_id(compressed, len);
        *buf = &buf[1 + compressed_size(len)..];
        Ok(Self(Arc::from(account_id)))
    }
}

static ALPHABET: &[u8; 39] = b".abcdefghijklmnopqrstuvwxyz0123456789-_";

fn char_index(c: u8) -> Option<usize> {
    ALPHABET.iter().position(|&x| x == c)
}

fn append_sub_byte(v: &mut [u8], start_bit: usize, sub_byte: u8, num_bits: usize) {
    assert!(num_bits <= 8);

    let sub_bits = sub_byte & (0b1111_1111 >> (8 - num_bits));

    let bit_offset = start_bit % 8;
    let keep_mask = !(0b1111_1111 << bit_offset)
        | !(0b1111_1111 >> (8usize.saturating_sub(num_bits + bit_offset)));
    let first_byte = (v[start_bit / 8] & keep_mask) | (sub_bits << bit_offset);

    v[start_bit / 8] = first_byte;

    if bit_offset + num_bits > 8 {
        let second_byte = sub_bits >> (8 - bit_offset);
        v[start_bit / 8 + 1] = second_byte;
    }
}

fn read_sub_byte(v: &[u8], start_bit: usize, num_bits: usize) -> u8 {
    assert!(num_bits <= 8);

    let bit_offset = start_bit % 8;
    let keep_mask = (0b1111_1111 << bit_offset)
        & (0b1111_1111 >> (8usize.saturating_sub(num_bits + bit_offset)));
    let first_byte = v[start_bit / 8] & keep_mask;

    let mut sub_byte = first_byte >> bit_offset;

    if bit_offset + num_bits > 8 {
        let num_bits_second = bit_offset + num_bits - 8;
        let second_byte = v[start_bit / 8 + 1];
        let keep_mask = 0b1111_1111 >> (8 - num_bits_second);
        sub_byte |= (second_byte & keep_mask) << (8 - bit_offset);
    }

    sub_byte
}

fn decompress_account_id(compressed: &[u8], len: usize) -> String {
    let mut s = String::with_capacity(len);
    for i in 0..len {
        let sub_byte = read_sub_byte(compressed, i * 6, 6);
        let c = ALPHABET[sub_byte as usize] as char;
        s.push(c);
    }
    s
}

fn compressed_size(len: usize) -> usize {
    len * 3 / 4 + (len * 3 % 4 > 0) as usize
}

fn compress_account_id(account_id: &str) -> Option<Vec<u8>> {
    let mut v = vec![0u8; compressed_size(account_id.len())];

    let mut i = 0;
    for c in account_id.as_bytes() {
        let index = char_index(*c).unwrap() as u8;
        append_sub_byte(&mut v, i, index, 6);
        i += 6;
    }

    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_sub_byte() {
        let mut v = vec![0u8; 2];
        append_sub_byte(&mut v, 0, 0b111, 3);
        append_sub_byte(&mut v, 3, 0b010, 3);
        append_sub_byte(&mut v, 6, 0b110, 3);
        append_sub_byte(&mut v, 9, 0b1110101, 7);

        assert_eq!(v, vec![0b10010111, 0b11101011]);
    }

    #[test]
    fn test_read_sub_byte() {
        let v = vec![0b10010111, 0b11101011];
        assert_eq!(read_sub_byte(&v, 0, 3), 0b111);
        assert_eq!(read_sub_byte(&v, 3, 3), 0b010);
        assert_eq!(read_sub_byte(&v, 6, 3), 0b110);
        assert_eq!(read_sub_byte(&v, 9, 7), 0b1110101);
    }

    #[test]
    fn test() {
        let account_id = "test.near";
        let compressed = compress_account_id(account_id).unwrap();
        let decompressed = decompress_account_id(&compressed, account_id.len());
        assert_eq!(account_id, decompressed);
    }

    #[test]
    fn test_account_id_borsh() {
        let account_id = "0".repeat(64);
        let account_id = FastAccountId(Arc::from(account_id));
        let serialized = account_id.try_to_vec().unwrap();
        let deserializalized = FastAccountId::try_from_slice(&serialized).unwrap();
        assert_eq!(account_id, deserializalized);
    }
}
