use near_contract_tools::Nep148;
use near_sdk::{json_types::Base64VecU8, near_bindgen};

#[derive(Nep148)]
#[nep148(
    name = "Test Fungible Token",
    symbol = "TFT",
    decimals = 18,
    icon = "https://example.com/icon.png",
    reference = "https://example.com/metadata.json",
    reference_hash = "YXNkZg=="
)]
#[near_bindgen]
struct DerivesFTMetadata {}

#[test]
fn test() {
    let ft = DerivesFTMetadata {};
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
