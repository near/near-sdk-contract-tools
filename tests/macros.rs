use near_contract_tools::{
    event::*,
    ownership::{Ownable, OwnershipController},
    Event, Ownable,
};
use near_sdk::{
    borsh::{self, BorshSerialize},
    env, near_bindgen,
    test_utils::VMContextBuilder,
    testing_env, AccountId, BorshStorageKey,
};
use serde::Serialize;

#[derive(BorshStorageKey, BorshSerialize)]
enum StorageKey {
    MyStorageKey,
}

#[derive(Ownable)]
#[ownable(storage_key = "StorageKey::MyStorageKey")]
#[near_bindgen]
pub struct OwnedStruct {
    pub permissioned_item: u32,
}

#[near_bindgen]
impl OwnedStruct {
    #[init]
    pub fn new() -> Self {
        let contract = Self {
            permissioned_item: 0,
        };

        // This method can only be called once throughout the entire duration of the contract
        contract.init_owner(env::predecessor_account_id());

        contract
    }

    pub fn set_permissioned_item(&mut self, value: u32) {
        self.require_owner();

        self.permissioned_item = value;
    }

    pub fn get_permissioned_item(&self) -> u32 {
        self.permissioned_item
    }
}

#[derive(Serialize)]
pub struct Nep171NftMintData {
    pub owner_id: String,
    pub token_ids: Vec<String>,
}

#[derive(Event, Serialize)]
// Required fields
#[event(standard = "nep171", version = "1.0.0")]
// Optional. Default event name is the untransformed variant name, e.g. NftMint, AnotherEvent, CustomEvent
#[event(rename_all = "snake_case")]
// Variant name will not appear in the serialized output
#[serde(untagged)]
pub enum Nep171 {
    NftMint(Vec<Nep171NftMintData>), // Name will be "nft_mint" because rename_all = snake_case

    #[event(name = "sneaky_event")]
    AnotherEvent, // Name will be "sneaky_event"

    #[event(rename = "SHOUTY-KEBAB-CASE")]
    CustomEvent, // Name will be "CUSTOM-EVENT"
}

#[test]
fn derive_event() {
    let e = Nep171::NftMint(vec![Nep171NftMintData {
        owner_id: "owner".to_string(),
        token_ids: vec!["token_1".to_string(), "token_2".to_string()],
    }]);
    println!("{}", e.to_event_string());
    assert_eq!(
        e.to_event_string(),
        r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_mint","data":[{"owner_id":"owner","token_ids":["token_1","token_2"]}]}"#
    );
}

#[test]
fn derive_ownable() {
    let owner: AccountId = "owner".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let mut c = OwnedStruct::new();

    assert_eq!(
        c.own_get_owner(),
        Some(owner.clone()),
        "Owner is initialized",
    );

    c.set_permissioned_item(4);
}

#[test]
#[should_panic(expected = "Owner only")]
fn derive_ownable_unauthorized() {
    let owner: AccountId = "owner".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(owner.clone())
        .build();

    testing_env!(context);
    let mut c = OwnedStruct::new();

    let alice: AccountId = "alice".parse().unwrap();
    let context = VMContextBuilder::new()
        .predecessor_account_id(alice.clone())
        .build();
    testing_env!(context);

    // Alice is not authorized to call owner-only method
    c.set_permissioned_item(4);
}
