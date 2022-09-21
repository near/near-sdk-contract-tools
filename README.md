# near-contract-tools

> Helpful functions and macros for developing smart contracts on NEAR Protocol.

See https://docs.rs/near-contract-tools/latest/near_contract_tools/ for more docs.

This package is a collection of common tools and patterns in NEAR smart contract development:

- Storage fee management
- Owner pattern (derive macro available)
- Role-based access control
- Pause (derive macro available)
- Derive macro for [NEP-297 events](https://nomicon.io/Standards/EventsFormat)
- Derive macro for [NEP-141](https://nomicon.io/Standards/Tokens/FungibleToken/Core) (and [NEP-148](https://nomicon.io/Standards/Tokens/FungibleToken/Metadata)) fungible tokens

Not to be confused with [`near-contract-standards`](https://crates.io/crates/near-contract-standards), which contains official implementations of standardized NEPs. This crate is intended to be a complement to `near-contract-standards`.

**WARNING:** This is still early software, and there may be breaking changes between versions. I'll try my best to keep the docs & changelogs up-to-date. Don't hesitate to create an issue if find anything wrong.

## Benefits

- requires fewer lines of code
  - Without near-contract-tools, implementing fungible token events (mint, transfer, and burn) takes ~100 lines of code. Using near-contract-tools, you can [implement them in ~40 lines](https://youtu.be/kJzes_UP5j0?t=1058).
- is more readable
- follows a consistent pattern
  - Every time you use the events macro, it will [implement events in the same way](https://youtu.be/kJzes_UP5j0?t=1150). Without it, you’d need to ensure that `emit`, `emit_many`, etc all work (and work the same).
- is more thorough
  - near-contract-standards is also not implementing traits, so that’s another improvement that near-contract-tools offers.

You can think of this collection of common tools and patterns (mostly in the form of [derive macros](https://doc.rust-lang.org/reference/procedural-macros.html#derive-macros)) as sort of an OpenZeppelin for NEAR.

## Getting Started

```text
rustup target add wasm32-unknown-unknown
cargo init
cargo add near-contract-tools
cargo add near-sdk
# https://raen.dev/guide/intro/getting-set-up.html
cargo install raen
# Implement a contract. See `workspaces-tests/src/bin/simple_multisig.rs` for example until we offer better examples here. Then:
near dev-deploy $(raen build --release -q)
```

### Example Usage

After installing NEAR CLI (https://docs.near.org/tools/near-cli#near-call), call like:

```text
near call dev-1662491554455-22903649156976 new --account-id example-acct-alice.testnet
near call dev-1662491554455-22903649156976 obtain_multisig_permission --account-id example-acct-alice.testnet
near call dev-1662491554455-22903649156976 request '{"action": "hello"}' --account-id example-acct-alice.testnet
near call dev-1662491554455-22903649156976 approve '{"request_id": 0}' --account-id example-acct-alice.testnet
near call dev-1662491554455-22903649156976 obtain_multisig_permission --account-id example-acct-bob.testnet
near call dev-1662491554455-22903649156976 approve '{"request_id": 0}' --account-id example-acct-bob.testnet
near call dev-1662491554455-22903649156976 execute '{"request_id": 0}' --account-id example-acct-bob.testnet
```

## Build and test

Install `cargo-make` if it is not installed already:

```text
cargo install cargo-make
```

Run tests:

```text
cargo test
cd workspaces-tests
cargo make test
```

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
        let mut contract = Self {
            // ...
        };

        Owner::init(&mut contract, &owner_id);

        contract
    }

    pub fn owner_only(&self) {
        Self::require_owner();

        // ...
    }
}
```

The `Owner` derive macro exposes the following methods to the blockchain:

```rust, ignore
fn own_get_owner(&self) -> Option<AccountId>;
fn own_get_proposed_owner(&self) -> Option<AccountId>;
fn own_renounce_owner(&mut self);
fn own_propose_owner(&mut self, account_id: Option<AccountId>);
fn own_accept_owner(&mut self);
```

### Events

```rust
use near_contract_tools::{event, standard::nep297::Event};

#[event(standard = "nep171", version = "1.0.0")]
pub struct MintEvent {
    pub owner_id: String,
    pub token_ids: Vec<String>,
}

let e = MintEvent {
    owner_id: "account".to_string(),
    token_ids: vec![
        "t1".to_string(),
        "t2".to_string(),
    ],
};

// Emits the event to the blockchain
e.emit();
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
    no_hooks
)]
#[near_bindgen]
struct FungibleToken {
    // ...
}
```

Standalone macros for each individual standard also exist.

### Macro Combinations

One may wish to combine the features of multiple macros in one contract. All of the macros are written such that they will work in a standalone manner, so this should largely work without issue. However, sometimes it may be desirable for the macros to work in _combination_ with each other. For example, to make a fungible token pausable, use the fungible token hooks to require that a contract be unpaused before making a token transfer:

```rust
use near_contract_tools::{
    pause::Pause,
    standard::nep141::{Nep141Hook, Nep141Transfer},
    FungibleToken, Pause,
};
use near_sdk::near_bindgen;

#[derive(FungibleToken, Pause)]
#[fungible_token(name = "Pausable Fungible Token", symbol = "PFT", decimals = 18)]
#[near_bindgen]
struct Contract {}

impl Nep141Hook for Contract {
    fn before_transfer(&mut self, _transfer: &Nep141Transfer) {
        Contract::require_unpaused();
    }
}
```

Note: Hooks can be disabled using `#[nep141(no_hooks)]` or `#[fungible_token(no_hooks)]`.

### Custom Crates

If you are a library developer, have modified a crate that one of the `near-contract-tools` macros uses (like `serde` or `near-sdk`), or are otherwise using a crate under a different name, you can specify crate names in macros like so:

```rust, ignore
#[event(
    // ...
    crate = "near_contract_tools",
    macros = "near_contract_tools_macros",
    serde = "serde",
)]
// ...

#[derive(Owner)]
#[owner(
    // ...
    near_sdk = "near_sdk",
)]
```

## Other Tips

### [Internal vs External Methods](https://youtu.be/kJzes_UP5j0?t=2172)

Internal methods are not available to be callable via the blockchain. External ones are public and can be called by other contracts.

### [Proposal Pattern](https://youtu.be/kJzes_UP5j0?t=2213)

Proposing ownership (rather than transferring directly) is a generally good practice because it prevents you from accidentally transferring ownership to an account that nobody has access to (which would kill the contract).

### [Expand](https://youtu.be/kJzes_UP5j0?t=1790)

`cargo expand` will generate one huge Rust file with all of the macros generated:

```text
cargo install cargo-expand
cargo expand > expanded.rs
```

### [Owner trait](https://youtu.be/kJzes_UP5j0?t=2520)

In order to implement the owner trait, you only have to implement one function: “root”.

### [Slots](https://youtu.be/kJzes_UP5j0?t=2527)

See [src/slot.rs](src/slot.rs) They are very thin wrappers over a storage key. It provides a sort of namespacing / key-combining functionality and also functionality such as "read", "write", "exists", "remove", etc.

### Reminders about NEAR functionality

- [assert_one_yocto()](https://youtu.be/kJzes_UP5j0?t=2989)

  `assert_one_yocto()` in near_sdk is a function that requires a full access key (by requiring a deposit of one yoctonear, the smallest possible unit of NEAR).

  Why is this important?

  If a user connects their NEAR account to a dapp and gives the dapp permissions to call functions on this smart contract on their behalf, the dapp _still_ will not be able to call _this function_ (i.e. any function that calls `assert_one_yocto()`) on their behalf.

  The only way to add this requirement (to force the transaction to be signed by a full access key) is to require some non-zero transfer.

## Contributing

### Getting Started

First, run `git config core.hooksPath hooks/` to install the hooks of this directory (without affecting how git hooks work for other projects).

## Authors

- Jacob Lindahl [@sudo_build](https://twitter.com/sudo_build)
