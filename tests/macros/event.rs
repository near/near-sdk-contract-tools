use near_sdk_contract_tools::standard::nep297::{Event, ToEventLog};

use crate::macros::event::test_events::Nep171NftMintData;

mod test_events {
    use near_sdk_contract_tools::Nep297;
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

    #[derive(Nep297, Serialize)]
    #[nep297(standard = "enum-event", version = "1.0.0")]
    #[allow(clippy::enum_variant_names)]
    pub enum EnumEvent {
        VariantOne,
        #[nep297(name = "genuine_variant_two")]
        VariantTwo(),
        #[nep297(rename = "SHOUTY_SNAKE_CASE")]
        VariantThree(u32, u64),
        #[nep297(rename = "kebab-case")]
        #[allow(unused)] // just here to make sure it compiles
        VariantFour {
            foo: u32,
            bar: u64,
        },
    }

    #[derive(Nep297, Serialize)]
    #[nep297(standard = "enum-event", version = "1.0.0", rename_all = "snake_case")]
    #[allow(clippy::enum_variant_names)]
    pub enum EnumEventRenameAll {
        VariantOne,
        #[nep297(rename = "lowerCamelCase")]
        VariantTwo,
        #[nep297(name = "threedom!")]
        VariantThree,
    }
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

    assert_eq!(
        test_events::AnotherEvent.to_event_log().event,
        "sneaky_event"
    );
    assert_eq!(
        test_events::CustomEvent.to_event_log().event,
        "CUSTOM-EVENT"
    );
    assert_eq!(
        test_events::EnumEvent::VariantOne.to_event_log().event,
        "VariantOne"
    );
    assert_eq!(
        test_events::EnumEvent::VariantTwo().to_event_log().event,
        "genuine_variant_two"
    );
    assert_eq!(
        test_events::EnumEvent::VariantThree(0, 0)
            .to_event_log()
            .event,
        "VARIANT_THREE"
    );
    assert_eq!(
        test_events::EnumEventRenameAll::VariantOne
            .to_event_log()
            .event,
        "variant_one"
    );
    assert_eq!(
        test_events::EnumEventRenameAll::VariantTwo
            .to_event_log()
            .event,
        "variantTwo"
    );
    assert_eq!(
        test_events::EnumEventRenameAll::VariantThree
            .to_event_log()
            .event,
        "threedom!"
    );
}

mod event_attribute_macro {
    use near_sdk_contract_tools::{event, standard::nep297::Event};

    mod my_event {
        use near_sdk_contract_tools::event;

        #[event(standard = "my_event_standard", version = "1")]
        pub struct One;
        #[event(standard = "my_event_standard", version = "1")]
        pub struct ThreePointFive {
            pub foo: &'static str,
        }
        #[event(standard = "my_event_standard", version = "1")]
        pub struct Six;
    }

    #[event(standard = "my_event_standard", version = "1")]
    #[allow(unused)]
    enum MyEvent {
        One,
        ThreePointFive { foo: &'static str },
        Six,
    }

    #[test]
    fn test() {
        let e = my_event::ThreePointFive { foo: "hello" };
        e.emit();
        assert_eq!(
            e.to_event_string(),
            r#"EVENT_JSON:{"standard":"my_event_standard","version":"1","event":"three_point_five","data":{"foo":"hello"}}"#,
        );

        let f = MyEvent::ThreePointFive { foo: "hello" };
        f.emit();
        assert_eq!(e.to_event_string(), f.to_event_string());
    }
}
