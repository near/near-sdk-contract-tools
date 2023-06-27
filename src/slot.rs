//! Managed storage slots
//!
//! Makes it easy to create and manage storage keys and avoid unnecessary
//! writes to contract storage. This reduces transaction IO  and saves on gas.
use std::marker::PhantomData;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, IntoStorageKey,
};

use crate::utils::prefix_key;

/// A storage slot, composed of a storage location (key) and a data type
#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct Slot<T> {
    /// The storage key this slot controls
    pub key: Vec<u8>,
    #[borsh_skip]
    _marker: PhantomData<T>,
}

impl Slot<()> {
    /// A placeholder slot. Useful for creating namespaced fields.
    pub fn root<K: IntoStorageKey>(key: K) -> Self {
        Self {
            key: key.into_storage_key(),
            _marker: PhantomData,
        }
    }
}

impl<T> Slot<T> {
    /// Creates a new [`Slot`] that controls the given storage key
    pub fn new(key: impl IntoStorageKey) -> Self {
        Self {
            key: key.into_storage_key(),
            _marker: PhantomData,
        }
    }

    /// Creates a new [`Slot`] that controls the given key namespaced (prefixed)
    /// by the parent key, to be used as a namespace for another subfield.
    pub fn ns(&self, key: impl IntoStorageKey) -> Slot<()> {
        Slot {
            key: prefix_key(&self.key, &key.into_storage_key()),
            _marker: PhantomData,
        }
    }

    /// Creates a new [`Slot`] that controls the given key namespaced (prefixed)
    /// by the parent key.
    pub fn field<U>(&self, key: impl IntoStorageKey) -> Slot<U> {
        Slot {
            key: prefix_key(&self.key, &key.into_storage_key()),
            _marker: PhantomData,
        }
    }

    /// Creates a [`Slot`] that tries to parse a different data type from the same
    /// storage slot.
    ///
    /// # Warning
    ///
    /// If the data in the slot is not parsable into the new type, methods like
    /// [`Slot::read`] and [`Slot::take`] will panic.
    pub fn transmute<U>(&self) -> Slot<U> {
        Slot {
            key: self.key.clone(),
            _marker: PhantomData,
        }
    }

    /// Write raw bytes into the storage slot. No type checking.
    pub fn write_raw(&mut self, value: &[u8]) -> bool {
        env::storage_write(&self.key, value)
    }

    /// Read raw bytes from the slot. No type checking or parsing.
    pub fn read_raw(&self) -> Option<Vec<u8>> {
        env::storage_read(&self.key)
    }

    /// Returns `true` if this slot's key is currently present in the smart
    /// contract storage, `false` otherwise
    pub fn exists(&self) -> bool {
        env::storage_has_key(&self.key)
    }

    /// Removes the managed key from storage
    pub fn remove(&mut self) -> bool {
        env::storage_remove(&self.key)
    }
}

impl<T: BorshSerialize> Slot<T> {
    /// Writes a value to the managed storage slot
    pub fn write(&mut self, value: &T) -> bool {
        self.write_raw(&value.try_to_vec().unwrap())
    }

    /// If the given value is `Some(T)`, writes `T` to storage. Otherwise,
    /// removes the key from storage.
    ///
    /// Use of this method makes the slot function similarly to
    /// [`near_sdk::collections::LazyOption`].
    pub fn set(&mut self, value: Option<&T>) -> bool {
        match value {
            Some(value) => self.write(value),
            _ => self.remove(),
        }
    }
}

impl<T: BorshDeserialize> Slot<T> {
    /// Reads a value from storage, if present.
    pub fn read(&self) -> Option<T> {
        self.read_raw().map(|v| T::try_from_slice(&v).unwrap())
    }

    /// Removes a value from storage and returns it if present.
    pub fn take(&mut self) -> Option<T> {
        if self.remove() {
            // unwrap should be safe if remove returns true
            Some(T::try_from_slice(&env::storage_get_evicted().unwrap()).unwrap())
        } else {
            None
        }
    }
}

impl<T: BorshSerialize + BorshDeserialize> Slot<T> {
    /// Writes a value to storage and returns the evicted value, if present.
    pub fn swap(&mut self, value: &T) -> Option<T> {
        if self.write_raw(&value.try_to_vec().unwrap()) {
            // unwrap should be safe because write_raw returned true
            Some(T::try_from_slice(&env::storage_get_evicted().unwrap()).unwrap())
        } else {
            None
        }
    }
}

impl<T> IntoStorageKey for Slot<T> {
    fn into_storage_key(self) -> Vec<u8> {
        self.key
    }
}

impl<T, U> PartialEq<Slot<U>> for Slot<T> {
    fn eq(&self, other: &Slot<U>) -> bool {
        self.key == other.key
    }
}

#[cfg(test)]
mod tests {
    use super::Slot;

    #[test]
    fn partialeq() {
        let a1 = Slot::<u32>::new(b"a");
        let a2 = Slot::<i32>::new(b"a");
        assert_eq!(a1, a2);
        let b = Slot::<u32>::new(b"b");
        assert_ne!(a1, b);
    }
}
