# near-contract-tools

> Helpful functions and macros for developing smart contracts on NEAR Protocol.

This package is a collection of common tools and patterns in NEAR smart contract development:

- Storage fee management
- Ownership pattern (derive macro available)
- Role-based access control
- Pausable (derive macro available)
- Derive macro for [NEP-297 events](https://nomicon.io/Standards/EventsFormat)

Not to be confused with [`near-contract-standards`](https://crates.io/crates/near-contract-standards), which contains official implementations of standardized NEPs. This crate is intended to be a complement to `near-contract-standards`.

**WARNING:** This is still early software, and there may be breaking changes between versions. I'll try my best to keep the docs & changelogs up-to-date. Don't hesitate to create an issue if find anything wrong.

## Example

### Ownership

```rust
use near_contract_tools::{Ownable, ownership::OwnershipController};
use near_sdk::{AccountId, near_bindgen};

#[derive(Ownable)]
#[near_bindgen]
struct Contract {
    // ...
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        let contract = Self {
            // ...
        };

        // Initialize the owner of the contract
        contract.init_owner(owner_id);

        contract
    }

    pub fn owner_only_method(&self) {
        self.require_owner();

        // ...
    }
}
```

This creates a smart contract which exposes the `Ownable` trait to the blockchain:

```rust
pub trait Ownable {
    fn own_get_owner(&self) -> Option<AccountId>;
    fn own_get_proposed_owner(&self) -> Option<AccountId>;
    fn own_renounce_owner(&mut self);
    fn own_propose_owner(&mut self, account_id: Option<AccountId>);
    fn own_accept_owner(&mut self);
}
```

### Events

```rust
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
    #[event(name = "nft_mint")]
    NftMint(Vec<Nep171NftMintData>),
}

let my_event = Nep171::NftMint(vec![Nep171NftMintData {
    owner_id: "owner".to_string(),
    token_ids: vec!["token_1".to_string(), "token_2".to_string()],
}]);

my_event.emit(); // Emits event to the blockchain
```

## Authors

- Jacob Lindahl [@sudo_build](https://twitter.com/sudo_build)
