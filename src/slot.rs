use std::marker::PhantomData;

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, IntoStorageKey,
};

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug)]
pub struct Slot<T> {
    pub key: Vec<u8>,
    #[borsh_skip]
    _marker: PhantomData<T>,
}

impl<T> Slot<T> {
    pub fn new<K: IntoStorageKey>(key: K) -> Self {
        Self {
            key: key.into_storage_key(),
            _marker: PhantomData,
        }
    }

    pub fn child<K: IntoStorageKey, U>(&self, key: K) -> Slot<U> {
        Slot {
            key: [self.key.clone(), key.into_storage_key()].concat(),
            _marker: PhantomData,
        }
    }

    pub unsafe fn transmute<U>(&self) -> Slot<U> {
        Slot {
            key: self.key.clone(),
            _marker: PhantomData,
        }
    }

    pub fn write_raw(&self, value: &[u8]) -> bool {
        env::storage_write(&self.key, value)
    }

    pub fn read_raw(&self) -> Option<Vec<u8>> {
        env::storage_read(&self.key)
    }

    pub fn exists(&self) -> bool {
        env::storage_has_key(&self.key)
    }

    pub fn remove(&self) -> bool {
        env::storage_remove(&self.key)
    }
}

impl<T: BorshSerialize> Slot<T> {
    pub fn write(&self, value: &T) -> bool {
        self.write_raw(&value.try_to_vec().unwrap())
    }

    pub fn set(&self, value: Option<&T>) -> bool {
        match value {
            Some(value) => self.write(value),
            _ => self.remove(),
        }
    }
}

impl<T: BorshDeserialize> Slot<T> {
    pub fn read(&self) -> Option<T> {
        self.read_raw().map(|v| T::try_from_slice(&v).unwrap())
    }

    pub fn take(&self) -> Option<T> {
        if self.remove() {
            // unwrap should be safe if remove returns true
            Some(T::try_from_slice(&env::storage_get_evicted().unwrap()).unwrap())
        } else {
            None
        }
    }
}

impl<T: BorshSerialize + BorshDeserialize> Slot<T> {
    pub fn swap(&self, value: T) -> Option<T> {
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
    fn test() {
        let a1 = Slot::<u32>::new(b"a");
        let a2 = Slot::<i32>::new(b"a");
        assert_eq!(a1, a2);
        let b = Slot::<u32>::new(b"b");
        assert_ne!(a1, b);
    }
}
