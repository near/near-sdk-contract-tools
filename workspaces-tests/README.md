# `workspaces` tests

This package contains tests for the `near-sdk-contract-tools` package using the `workspaces` crate.

## Running the tests

1. Ensure that the Cargo extension `cargo-make` is installed: `cargo install cargo-make`
2. Run `cargo make test`

## Creating a new test

If you wish to create a new test in this package, create a new file under the `tests` directory with the name of your test.

If your test requires the creation of new smart contracts, follow these steps for each new contract:

1. Create the file in `src/bin/<contract-name>.rs` where `<contract-name>` is the name of the new smart contract.
2. Add an entry to `Cargo.toml` like so:
    ```toml
    [[bin]]
    name = "<contract-name>"
    ```
3. The new contract must contain a `main` method, although it will not be used. This is due to [limitations](https://doc.rust-lang.org/cargo/reference/cargo-targets.html#library) on how Cargo handles packages with multiple entry points. Unfortunately, `#![no_main]` does not work for these purposes. Therefore, `src/bin/<contract-name>.rs` must contain the following:
    ```rust
    pub fn main() {}
    ```
4. Compile the smart contract using the following command:
    ```text
    cargo build --target wasm32-unknown-unknown --release --bin <contract-name>
    ```
5. Include the compiled contract in the test file:
    ```rust
    include_bytes!("../../target/wasm32-unknown-unknown/release/<contract-name>.wasm")
    ```
