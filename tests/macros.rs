use near_contract_tools::event::*;
use near_contract_tools::Event;
use serde::Serialize;

#[derive(Serialize)]
pub struct Nep171NftMintData {
    pub owner_id: String,
    pub token_ids: Vec<String>,
}

#[derive(Event, Serialize)]
#[event(standard = "nep171", version = "1.0.0")]
#[serde(untagged)]
pub enum Nep171 {
    #[event = "nft_mint"]
    NftMint(Vec<Nep171NftMintData>),
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
}
