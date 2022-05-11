# near-contract-tools

> Helpful functions and macros for developing smart contracts on NEAR Protocol.

This package is a collection of common tools and patterns in NEAR smart contract development:

- Storage fee management
- Ownership pattern
- Role-based access control

Not to be confused with [`near-contract-standards`](https://crates.io/crates/near-contract-standards), which contains official implementations of standardized NEPs.

## Example

```rust
use near_sdk::{
    near_bindgen,
    AccountId,
    assert_one_yocto,
};
use near_contract_tools::{
    impl_ownership,
    ownership::Ownership,
};

#[near_bindgen]
struct Contract {
    pub ownership: Ownership;
}

impl_ownership!(Contract, ownership);
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

## Authors

- Jacob Lindahl [@sudo_build](https://twitter.com/sudo_build)
