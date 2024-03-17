//! A fast alternative to `near_sdk::AccountId` that is faster to use, and has a
//! smaller Borsh serialization footprint.

use std::{ops::Deref, rc::Rc, str::FromStr};

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

static ALPHABET: &[u8; 39] = b".abcdefghijklmnopqrstuvwxyz0123456789-_";

const fn char_index(c: u8) -> Option<usize> {
    match c {
        b'.' => Some(0),
        b'a'..=b'z' => Some((1 + c - b'a') as usize),
        b'0'..=b'9' => Some((27 + c - b'0') as usize),
        b'-' => Some(37),
        b'_' => Some(38),
        _ => None,
    }
}

/// An alternative to `near_sdk::AccountId` that is faster to use, and has a
/// smaller Borsh serialization footprint.
///
/// Limitations:
///  - Does not implement `serde` serialization traits.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FastAccountId(Rc<str>);

impl FastAccountId {
    /// Creates a new `FastAccountId` from a `&str` without performing any checks.
    pub fn new_unchecked(account_id: &str) -> Self {
        Self(Rc::from(account_id))
    }
}

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
        Self(Rc::from(account_id.as_str()))
    }
}

#[cfg(feature = "near-sdk-4")]
impl From<FastAccountId> for near_sdk::AccountId {
    fn from(account_id: FastAccountId) -> Self {
        Self::new_unchecked(account_id.0.to_string())
    }
}

impl FromStr for FastAccountId {
    type Err = <near_sdk::AccountId as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        near_sdk::AccountId::from_str(s).map(Self::from)
    }
}

impl TryFrom<&str> for FastAccountId {
    type Error = <near_sdk::AccountId as FromStr>::Err;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        near_sdk::AccountId::from_str(s).map(Self::from)
    }
}

impl BorshSerialize for FastAccountId {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let len: u8 = self.0.len() as u8;
        writer.write_all(&[len])?;
        let compressed = compress_account_id(&self.0).ok_or(std::io::ErrorKind::InvalidData)?;
        writer.write_all(&compressed)?;
        Ok(())
    }
}

impl BorshDeserialize for FastAccountId {
    #[cfg(feature = "near-sdk-4")]
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let len = buf[0] as usize;
        let compressed = &buf[1..];
        let account_id = decompress_account_id(compressed, len);
        *buf = &buf[1 + compressed_size(len)..];
        Ok(Self(Rc::from(account_id)))
    }

    #[cfg(feature = "near-sdk-5")]
    fn deserialize_reader<R: std::io::Read>(buf: &mut R) -> std::io::Result<Self> {
        let mut l = [0u8];
        buf.read_exact(&mut l)?;
        let len = l[0] as usize;
        let mut compressed = [0u8; near_sdk::AccountId::MAX_LEN];
        buf.read_exact(&mut compressed[..compressed_size(len)])?;
        let account_id = decompress_account_id(&compressed, len);
        Ok(Self(Rc::from(account_id)))
    }
}

fn append_sub_byte(v: &mut [u8], start_bit: usize, sub_byte: u8, num_bits: usize) {
    assert!(num_bits <= 8);

    let sub_bits = sub_byte & (0b1111_1111 >> (8 - num_bits));

    let bit_offset = start_bit % 8;
    let keep_mask = !select_bits_mask(bit_offset, num_bits);
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
    let keep_mask = select_bits_mask(bit_offset, num_bits);
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

const fn select_bits_mask(start_bit_index: usize, num_bits: usize) -> u8 {
    (0b1111_1111 << start_bit_index)
        & (0b1111_1111 >> (8usize.saturating_sub(num_bits + start_bit_index)))
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

const fn compressed_size(len: usize) -> usize {
    #[cfg(feature = "near-sdk-5")]
    debug_assert!(
        len <= near_sdk::AccountId::MAX_LEN,
        "Account ID is too long"
    );
    len * 3 / 4 + (len * 3 % 4 > 0) as usize
}

fn compress_account_id(account_id: &str) -> Option<Vec<u8>> {
    let mut v = vec![0u8; compressed_size(account_id.len())];

    let mut i = 0;
    for c in account_id.as_bytes() {
        let index = char_index(*c)? as u8;
        append_sub_byte(&mut v, i, index, 6);
        i += 6;
    }

    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_index() {
        // because char_index() is implemented using a match so that it is const
        for c in ALPHABET {
            assert_eq!(char_index(*c), ALPHABET.iter().position(|d| d == c));
        }

        assert!(char_index(b'A').is_none());
    }

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
    fn test_compression_decompression() {
        let account_id = "test.near";
        let compressed = compress_account_id(account_id).unwrap();
        assert_eq!(compressed.len(), 7);
        let decompressed = decompress_account_id(&compressed, account_id.len());
        assert_eq!(account_id, decompressed);
    }

    #[test]
    fn test_account_id_borsh() {
        let account_id = "0".repeat(64);
        let sdk_account_id = near_sdk::AccountId::from_str(&account_id).unwrap();
        let expected_serialized_length = 64 * 3 / 4 + 1; // no +1 for remainder (64 * 3 % 4 == 0), but +1 for length
        let account_id = FastAccountId::new_unchecked(&account_id);
        let serialized = compat_borsh_serialize!(&account_id).unwrap();
        assert_eq!(serialized.len(), expected_serialized_length);
        let deserializalized = FastAccountId::try_from_slice(&serialized).unwrap();
        assert_eq!(account_id, deserializalized);

        let sdk_serialized = compat_borsh_serialize!(&sdk_account_id).unwrap();
        assert!(sdk_serialized.len() > serialized.len()); // gottem
    }

    #[test]
    fn various_serializations() {
        let tests = [
            "",
            "1",
            "a",
            "abcdef",
            "a.b.c.d",
            "root.near",
            "system",
            "near",
            "a_b-cdefghijklmnopqrstuvwxy.z0123456789",
            "0000000000000000000000000000000000000000000000000000000000000000",
        ];

        for test in tests {
            let account_id = FastAccountId::new_unchecked(test);
            let serialized = compat_borsh_serialize!(&account_id).unwrap();
            let deserializalized = FastAccountId::try_from_slice(&serialized).unwrap();
            assert_eq!(account_id, deserializalized);
        }
    }
}
