pub mod fungible_token;
pub mod nep141;
pub mod nep148;

mod t {
    use near_contract_tools::{FungibleToken, Pause, pause::Pause, standard::nep141::{Nep141Controller, Nep141Hook}};
    use near_sdk::near_bindgen;

    #[derive(FungibleToken, Pause)]
    #[fungible_token(
        name = "Pausable Fungible Token",
        symbol = "PFT",
        decimals = 18,
        hook,
        // before_transfer = "Self::require_unpaused",
    )]
    #[near_bindgen]
    struct Contract {}

    impl Nep141Hook for Contract {
        fn before_transfer() {
            Self::require_unpaused();
        }
    }

    #[test]
    fn x() {
        Contract::before_transfer();
    }
}
