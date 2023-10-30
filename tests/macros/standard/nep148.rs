use near_sdk::{json_types::Base64VecU8, near_bindgen};
use near_sdk_contract_tools::{standard::nep148::*, Nep148};

#[derive(Nep148)]
#[near_bindgen]
struct DerivesFTMetadata {}

#[near_bindgen]
impl DerivesFTMetadata {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self {};

        contract.set_metadata(
            &FungibleTokenMetadata::new("Test Fungible Token".into(), "TFT".into(), 18)
                .icon("https://example.com/icon.png".into())
                .reference("https://example.com/metadata.json".into())
                .reference_hash(Base64VecU8::from([97, 115, 100, 102].to_vec())),
        );

        contract
    }
}

#[test]
fn test() {
    let ft = DerivesFTMetadata::new();
    let meta = ft.ft_metadata();
    println!("{:?}", &meta);
    assert_eq!(meta.decimals, 18);
    assert_eq!(meta.name, "Test Fungible Token");
    assert_eq!(meta.symbol, "TFT");
    assert_eq!(meta.icon, Some("https://example.com/icon.png".into()));
    assert_eq!(
        meta.reference,
        Some("https://example.com/metadata.json".into())
    );
    assert_eq!(
        meta.reference_hash,
        Some(Base64VecU8::from([97, 115, 100, 102].to_vec()))
    );
}
