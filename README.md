# near-sdk-contract-tools

## NFT

```rust
use near_sdk::near;
use near_sdk_contract_tools::nft::*;

#[derive(Default, NonFungibleToken)]
#[near(contract_state)]
pub struct MyNftContract {}

#[near]
impl MyNftContract {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self {};

        contract.set_contract_metadata(&ContractMetadata::new("My NFT", "MNFT", None));

        contract
    }
}
```

## FT

```rust
use near_sdk::near;
use near_sdk_contract_tools::ft::*;

#[derive(Default, FungibleToken)]
#[near(contract_state)]
pub struct MyFtContract {}

#[near]
impl MyFtContract {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self {};

        contract.set_metadata(&ContractMetadata::new("My Fungible Token", "MYFT", 24));

        contract
    }
}
```

## What is it?

This package is a collection of common tools and patterns in NEAR smart contract development:

- Storage fee management.
- Escrow pattern and derive macro.
- Owner pattern and derive macro.
- Pause pattern and derive macro.
- Role-based access control.
- Derive macros for NEP standards:
  - [NEP-141][nep141] (fungible token), extension [NEP-148][nep148].
  - [NEP-145][nep145] (storage management), and integrations for the fungible token and non-fungible token standards.
  - [NEP-171][nep171] (non-fungible token), extensions [NEP-177][nep177], [NEP-178][nep178], [NEP-181][nep181].
  - [NEP-245][nep245] (multi token), extensions [Metadata][nep245metadata], [Events (integrated)][nep245events].
  - [NEP-297][nep297] (events).

Not to be confused with [`near-contract-standards`](https://crates.io/crates/near-contract-standards), which contains official implementations of standardized NEPs. This crate is intended to be a complement to `near-contract-standards`.

You can think of this collection of common tools and patterns (mostly in the form of [derive macros](https://doc.rust-lang.org/reference/procedural-macros.html#derive-macros)) as a sort of [OpenZeppelin](https://docs.openzeppelin.com/contracts/4.x/) for NEAR.

## Installation

```bash
cargo add near-sdk-contract-tools
```

## Examples

See also: [the full integration tests](tests/macros/mod.rs).

### Owner

```rust
use near_sdk::{near, AccountId, PanicOnDefault};
use near_sdk_contract_tools::{owner::Owner, Owner};

#[derive(Owner, PanicOnDefault)]
#[near(contract_state)]
struct Contract {
    // ...
}

#[near]
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

The `#[event]` macro can be applied to structs or enums.

```rust
use near_sdk_contract_tools::{event, standard::nep297::Event};

#[event(standard = "nft", version = "1.0.0")]
pub struct MintEvent {
    pub owner_id: String,
    pub token_id: String,
}

let e = MintEvent {
    owner_id: "account".to_string(),
    token_id: "token_1".to_string(),
};

// Emits the event to the blockchain
e.emit();
```

### Fungible Token

To create a contract that is compatible with the [NEP-141][nep141], [NEP-145][nep145], and [NEP-148][nep148] standards, that emits standard-compliant ([NEP-297][nep297]) events.

```rust
use near_sdk::near;
use near_sdk_contract_tools::ft::*;

#[derive(Default, FungibleToken)]
#[near(contract_state)]
struct MyFt {}

#[near]
impl MyFt {
    #[init]
    pub fn new() -> Self {
        let mut contract = Self {};

        contract.set_metadata(&ContractMetadata::new("My Fungible Token", "MYFT", 24));

        contract
    }
}
```

Standalone macros for each individual standard also exist.

### Non-fungible Token

Use the `NonFungibleToken` derive macro to implement [NEP-145][nep145], [NEP-171][nep171], [NEP-177][nep177], [NEP-178][nep178], and [NEP-181][nep181], with [NEP-297][nep297] events.

```rust
use near_sdk::{near, PanicOnDefault};
use near_sdk_contract_tools::nft::*;

#[derive(NonFungibleToken, PanicOnDefault)]
#[near(contract_state)]
pub struct MyNft {}
```

### Macro Combinations

One may wish to combine the features of multiple macros in one contract. All of the macros are written such that they will work in a standalone manner, so this should largely work without issue. However, sometimes it may be desirable for the macros to work in _combination_ with each other. For example, to make a fungible token pausable, use the fungible token hooks to require that a contract be unpaused before making a token transfer:

```rust
use near_sdk_contract_tools::{
    ft::*,
    pause::{*, hooks::Pausable},
    Pause,
};
use near_sdk::{near, PanicOnDefault};

#[derive(FungibleToken, Pause, PanicOnDefault)]
#[fungible_token(all_hooks = "Pausable")]
#[near(contract_state)]
struct Contract {}
```

### Custom Crates

If you are a library developer, have modified a crate that one of the `near-sdk-contract-tools` macros uses (like `serde` or `near-sdk`), or are otherwise using a crate under a different name, you can specify crate names in macros like so:

```rust, ignore
#[event(
    // ...
    crate = "near_sdk_contract_tools",
    macros = "near_sdk_contract_tools_macros",
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

### [Internal vs. External Methods](https://youtu.be/kJzes_UP5j0?t=2172)

Internal methods are not available to be callable via the blockchain. External ones are public and can be called by other contracts.

### [Pull pattern](https://youtu.be/kJzes_UP5j0?t=2213)

Proposing ownership (rather than transferring directly) is a generally good practice because it prevents you from accidentally transferring ownership to an account that nobody has access to, bricking the contract.

### [Expand](https://youtu.be/kJzes_UP5j0?t=1790)

[`cargo expand`](https://crates.io/crates/cargo-expand) will generate one huge Rust file with all of the macros have been processed:

```text
cargo install cargo-expand
cargo expand > expanded.rs
```

### [Slots](https://youtu.be/kJzes_UP5j0?t=2527)

See [`src/slot.rs`](src/slot.rs). Slots are thin wrappers over a storage key.

### [`assert_one_yocto()`](https://youtu.be/kJzes_UP5j0?t=2989)

`near_sdk::assert_one_yocto()` is a function that requires a full access key (by requiring a deposit of one yoctonear, the smallest possible unit of NEAR).

If a user connects their NEAR account to a dapp, the dapp will still not be able to call functions that call `assert_one_yocto()`, since function call access keys are not allowed to transfer native tokens. These function will require a signature from a full access key, usually involving a confirmation screen in the user's wallet.

## Contributing

### Setup

Run `git config core.hooksPath hooks/` to set up commit hooks.

### Build and test

Install `cargo-make` if it is not installed already:

```text
cargo install cargo-make cargo-nextest
```

Run tests:

```text
cargo nextest run
cargo test --doc
cd workspaces-tests
cargo make nextest
```

## Audit

Version 1.0.0 of this library has been [audited](./documents/NEAR%20Contract%20Tools%20-%20Final%20-%2005.05.2023.pdf) by [Kudelski Security](https://www.kudelskisecurity.com/). (May 2023)

## Authors

- Jacob Lindahl [@sudo_build](https://twitter.com/sudo_build)

---

(Formerly known as [`near-contract-tools`](https://crates.io/crates/near-contract-tools).)

[nep141]: https://github.com/near/NEPs/blob/master/neps/nep-0141.md
[nep145]: https://github.com/near/NEPs/blob/master/neps/nep-0145.md
[nep148]: https://github.com/near/NEPs/blob/master/neps/nep-0148.md
[nep171]: https://github.com/near/NEPs/blob/master/neps/nep-0171.md
[nep177]: https://github.com/near/NEPs/blob/master/neps/nep-0177.md
[nep178]: https://github.com/near/NEPs/blob/master/neps/nep-0178.md
[nep181]: https://github.com/near/NEPs/blob/master/neps/nep-0181.md
[nep245]: https://github.com/near/NEPs/blob/master/neps/nep-0245.md
[nep245metadata]: https://github.com/near/NEPs/blob/master/neps/nep-0245/Metadata.md
[nep245events]: https://github.com/near/NEPs/blob/master/neps/nep-0245/Events.md
[nep297]: https://github.com/near/NEPs/blob/master/neps/nep-0297.md
