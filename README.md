# near-contract-tools

> Helpful functions and macros for developing smart contracts on NEAR Protocol.

This package is a collection of common tools and patterns in NEAR smart contract development:

- Storage fee management
- Owner pattern (derive macro available)
- Role-based access control
- Pause (derive macro available)
- Derive macro for [NEP-297 events](https://nomicon.io/Standards/EventsFormat)
- Derive macro for [NEP-141](https://nomicon.io/Standards/Tokens/FungibleToken/Core) (and [NEP-148](https://nomicon.io/Standards/Tokens/FungibleToken/Metadata)) fungible tokens

Not to be confused with [`near-contract-standards`](https://crates.io/crates/near-contract-standards), which contains official implementations of standardized NEPs. This crate is intended to be a complement to `near-contract-standards`.

**WARNING:** This is still early software, and there may be breaking changes between versions. I'll try my best to keep the docs & changelogs up-to-date. Don't hesitate to create an issue if find anything wrong.

## Examples

See also: [the full integration tests](tests/macros/mod.rs).

### Owner

```rust
use near_sdk::{near_bindgen, AccountId};
use near_contract_tools::{owner::Owner, Owner};

#[derive(Owner)]
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

        Owner::init(&contract, &owner_id);

        contract
    }

    pub fn owner_only(&self) {
        Self::require_owner();

        // ...
    }
}
```

The `Owner` derive macro exposes the following methods to the blockchain:

```rust
fn own_get_owner(&self) -> Option<AccountId>;
fn own_get_proposed_owner(&self) -> Option<AccountId>;
fn own_renounce_owner(&mut self);
fn own_propose_owner(&mut self, account_id: Option<AccountId>);
fn own_accept_owner(&mut self);
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

### Fungible Token

To create a contract that is compatible with the NEP-141 and NEP-148 standards, that emits standard-compliant (NEP-141, NEP-297) events.

```rust
use near_contract_tools::FungibleToken;
use near_sdk::near_bindgen;

#[derive(FungibleToken)]
#[fungible_token(
    name = "My Fungible Token",
    symbol = "MYFT",
    decimals = 18,
)]
#[near_bindgen]
struct FungibleToken {
    // ...
}
```

Standalone macros for each individual standard also exist.

## Authors

- Jacob Lindahl [@sudo_build](https://twitter.com/sudo_build)
