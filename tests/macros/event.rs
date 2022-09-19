use near_contract_tools::{event::*, Event};
use serde::Serialize;

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

    assert_eq!(
        e.to_event_string(),
        r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_mint","data":[{"owner_id":"owner","token_ids":["token_1","token_2"]}]}"#
    );

    assert_eq!(Nep171::AnotherEvent.event(), "sneaky_event");

    assert_eq!(Nep171::CustomEvent.event(), "CUSTOM-EVENT");
}

mod event_attribute_macro {
    use near_contract_tools::{event::Event, to_event};

    #[to_event(standard = "my_event_standard", version = "1")]
    #[allow(unused)]
    enum MyEvent {
        One,
        ThreePointFive { foo: &'static str },
        Six,
    }

    #[test]
    fn test() {
        let e = MyEvent::ThreePointFive { foo: "hello" };
        e.emit();
        assert_eq!(
            e.to_string(),
            r#"EVENT_JSON:{"standard":"my_event_standard","version":"1","event":"three_point_five","data":{"foo":"hello"}}"#,
        );
    }
}
