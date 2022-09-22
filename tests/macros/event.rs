use near_contract_tools::standard::nep297::Event;

use crate::macros::event::test_events::Nep171NftMintData;

mod test_events {
    use near_contract_tools::Nep297;
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct Nep171NftMintData {
        pub owner_id: String,
        pub token_ids: Vec<String>,
    }

    #[derive(Nep297, Serialize)]
    // Required fields
    #[nep297(standard = "nep171", version = "1.0.0")]
    // Optional. Default event name is the untransformed variant name, e.g. NftMint, AnotherEvent, CustomEvent
    #[nep297(rename = "snake_case")]
    pub struct NftMint(pub Vec<Nep171NftMintData>); // Name will be "nft_mint" because rename = snake_case

    #[derive(Nep297, Serialize)]
    #[nep297(standard = "nep171", version = "1.0.0", name = "sneaky_event")]
    pub struct AnotherEvent; // Name will be "sneaky_event"

    #[derive(Nep297, Serialize)]
    #[nep297(standard = "nep171", version = "1.0.0", rename = "SHOUTY-KEBAB-CASE")]
    pub struct CustomEvent; // Name will be "CUSTOM-EVENT"
}

#[test]
fn derive_event() {
    let e = test_events::NftMint(vec![Nep171NftMintData {
        owner_id: "owner".to_string(),
        token_ids: vec!["token_1".to_string(), "token_2".to_string()],
    }]);

    assert_eq!(
        e.to_event_string(),
        r#"EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_mint","data":[{"owner_id":"owner","token_ids":["token_1","token_2"]}]}"#
    );

    assert_eq!(test_events::AnotherEvent.event_log().event, "sneaky_event");

    assert_eq!(test_events::CustomEvent.event_log().event, "CUSTOM-EVENT");
}

mod event_attribute_macro {
    use near_contract_tools::standard::nep297::Event;

    mod my_event {
        use near_contract_tools::event;

        #[event(standard = "my_event_standard", version = "1")]
        pub struct One;
        #[event(standard = "my_event_standard", version = "1")]
        pub struct ThreePointFive {
            pub foo: &'static str,
        }
        #[event(standard = "my_event_standard", version = "1")]
        pub struct Six;
    }

    #[test]
    fn test() {
        let e = my_event::ThreePointFive { foo: "hello" };
        e.emit();
        assert_eq!(
            e.to_event_string(),
            r#"EVENT_JSON:{"standard":"my_event_standard","version":"1","event":"three_point_five","data":{"foo":"hello"}}"#,
        );
    }
}
