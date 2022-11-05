#![allow(missing_docs)]
#![no_main]
use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use near_contract_tools::slot::Slot;
use near_sdk::{test_utils::VMContextBuilder, testing_env};

#[derive(Arbitrary, Debug)]
enum Input {
    Store {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    StoreField {
        key: Vec<u8>,
        value_at_key: Vec<u8>,
        field: Vec<u8>,
        value_at_field: Vec<u8>,
    },
    StoreNamespaceField {
        key: Vec<u8>,
        value_at_key: Vec<u8>,
        namespace: Vec<u8>,
        field: Vec<u8>,
        value_at_field: Vec<u8>,
    },
}

fn fuzz(input: Input) {
    testing_env!(VMContextBuilder::new()
        .prepaid_gas(near_sdk::Gas::ONE_TERA * 30)
        .build());

    match input {
        Input::Store { key, value } => {
            let mut slot = Slot::<Vec<u8>>::new(key);
            assert!(slot.read().is_none());
            slot.write(&value);
            assert!(slot.read().is_some());
            assert_eq!(slot.read().as_ref(), Some(&value));
            slot.remove();
            assert!(slot.read().is_none());
        }
        Input::StoreField {
            key,
            value_at_key,
            field,
            value_at_field,
        } => {
            if field.len() == 0 {
                return;
            }

            let mut slot = Slot::<Vec<u8>>::new(key);
            let mut field: Slot<Vec<u8>> = slot.field(field);

            assert!(slot.read().is_none());
            slot.write(&value_at_key);
            assert!(slot.read().is_some());
            assert_eq!(slot.read().as_ref(), Some(&value_at_key));

            assert!(field.read().is_none());
            field.write(&value_at_field);
            assert!(field.read().is_some());
            assert_eq!(field.read().as_ref(), Some(&value_at_field));

            slot.remove();
            assert!(slot.read().is_none());

            assert!(field.read().is_some());
            assert_eq!(field.read().as_ref(), Some(&value_at_field));
            field.remove();
            assert!(field.read().is_none());
        }
        Input::StoreNamespaceField {
            key,
            value_at_key,
            namespace,
            field,
            value_at_field,
        } => {
            if field.len() == 0 {
                return;
            }

            let mut slot = Slot::<Vec<u8>>::new(key);
            let mut field: Slot<Vec<u8>> = slot.ns(namespace).field(field);

            assert!(slot.read().is_none());
            slot.write(&value_at_key);
            assert!(slot.read().is_some());
            assert_eq!(slot.read().as_ref(), Some(&value_at_key));

            assert!(field.read().is_none());
            field.write(&value_at_field);
            assert!(field.read().is_some());
            assert_eq!(field.read().as_ref(), Some(&value_at_field));

            slot.remove();
            assert!(slot.read().is_none());

            assert!(field.read().is_some());
            assert_eq!(field.read().as_ref(), Some(&value_at_field));
            field.remove();
            assert!(field.read().is_none());
        }
    }
}

fuzz_target!(|input: Input| { fuzz(input) });
