//! NEP-141 lifecycle hooks.

use near_sdk::AccountId;

use super::Nep141Transfer;

/// Contracts may implement this trait to inject code into NEP-141 functions.
///
/// `T` is an optional value for passing state between different lifecycle
/// hooks. This may be useful for charging callers for storage usage, for
/// example.
pub trait Nep141Hook<C = Self> {
    /// State value returned by [`Nep141Hook::before_mint`].
    type MintState;
    /// State value returned by [`Nep141Hook::before_transfer`].
    type TransferState;
    /// State value returned by [`Nep141Hook::before_burn`].
    type BurnState;

    /// Executed before a token mint is conducted.
    ///
    /// May return an optional state value which will be passed along to the
    /// following [`Nep141Hook::after_mint`].
    fn before_mint(contract: &C, amount: u128, account_id: &AccountId) -> Self::MintState;

    /// Executed after a token mint is conducted.
    ///
    /// Receives the state value returned by [`Nep141Hook::before_mint`].
    fn after_mint(contract: &mut C, amount: u128, account_id: &AccountId, state: Self::MintState);

    /// Executed before a token transfer is conducted.
    ///
    /// May return an optional state value which will be passed along to the
    /// following [`Nep141Hook::after_transfer`].
    fn before_transfer(contract: &C, transfer: &Nep141Transfer) -> Self::TransferState;

    /// Executed after a token transfer is conducted.
    ///
    /// Receives the state value returned by [`Nep141Hook::before_transfer`].
    fn after_transfer(contract: &mut C, transfer: &Nep141Transfer, state: Self::TransferState);

    /// Executed before a token burn is conducted.
    ///
    /// May return an optional state value which will be passed along to the
    /// following [`Nep141Hook::after_burn`].
    fn before_burn(contract: &C, amount: u128, account_id: &AccountId) -> Self::BurnState;

    /// Executed after a token burn is conducted.
    ///
    /// Receives the state value returned by [`Nep141Hook::before_burn`].
    fn after_burn(contract: &mut C, amount: u128, account_id: &AccountId, state: Self::BurnState);
}

impl<C> Nep141Hook<C> for () {
    type MintState = ();
    type TransferState = ();
    type BurnState = ();

    fn before_mint(_contract: &C, _amount: u128, _account_id: &AccountId) {}

    fn after_mint(_contract: &mut C, _amount: u128, _account_id: &AccountId, _: ()) {}

    fn before_transfer(_contract: &C, _transfer: &Nep141Transfer) {}

    fn after_transfer(_contract: &mut C, _transfer: &Nep141Transfer, _: ()) {}

    fn before_burn(_contract: &C, _amount: u128, _account_id: &AccountId) {}

    fn after_burn(_contract: &mut C, _amount: u128, _account_id: &AccountId, _: ()) {}
}

impl<C, T, U> Nep141Hook<C> for (T, U)
where
    T: Nep141Hook<C>,
    U: Nep141Hook<C>,
{
    type MintState = (T::MintState, U::MintState);
    type TransferState = (T::TransferState, U::TransferState);

    type BurnState = (T::BurnState, U::BurnState);

    fn before_mint(contract: &C, amount: u128, account_id: &AccountId) -> Self::MintState {
        (
            T::before_mint(contract, amount, account_id),
            U::before_mint(contract, amount, account_id),
        )
    }

    fn after_mint(
        contract: &mut C,
        amount: u128,
        account_id: &AccountId,
        (t_state, u_state): Self::MintState,
    ) {
        T::after_mint(contract, amount, account_id, t_state);
        U::after_mint(contract, amount, account_id, u_state);
    }

    fn before_transfer(contract: &C, transfer: &Nep141Transfer) -> Self::TransferState {
        (
            T::before_transfer(contract, transfer),
            U::before_transfer(contract, transfer),
        )
    }

    fn after_transfer(
        contract: &mut C,
        transfer: &Nep141Transfer,
        (t_state, u_state): Self::TransferState,
    ) {
        T::after_transfer(contract, transfer, t_state);
        U::after_transfer(contract, transfer, u_state);
    }

    fn before_burn(contract: &C, amount: u128, account_id: &AccountId) -> Self::BurnState {
        (
            T::before_burn(contract, amount, account_id),
            U::before_burn(contract, amount, account_id),
        )
    }

    fn after_burn(
        contract: &mut C,
        amount: u128,
        account_id: &AccountId,
        (t_state, u_state): Self::BurnState,
    ) {
        T::after_burn(contract, amount, account_id, t_state);
        U::after_burn(contract, amount, account_id, u_state);
    }
}

/// Alternative to [`Nep141Hook`] that allows for simpler hook implementations.
pub trait SimpleNep141Hook {
    /// Executed before a token mint is conducted.
    fn before_mint(&self, _amount: u128, _account_id: &AccountId) {}
    /// Executed after a token mint is conducted.
    fn after_mint(&mut self, _amount: u128, _account_id: &AccountId) {}

    /// Executed before a token transfer is conducted.
    fn before_transfer(&self, _transfer: &Nep141Transfer) {}
    /// Executed after a token transfer is conducted.
    fn after_transfer(&mut self, _transfer: &Nep141Transfer) {}

    /// Executed before a token burn is conducted.
    fn before_burn(&self, _amount: u128, _account_id: &AccountId) {}
    /// Executed after a token burn is conducted.
    fn after_burn(&mut self, _amount: u128, _account_id: &AccountId) {}
}

impl<C: SimpleNep141Hook> Nep141Hook<C> for C {
    type MintState = ();

    type TransferState = ();

    type BurnState = ();

    fn before_mint(contract: &C, amount: u128, account_id: &AccountId) {
        SimpleNep141Hook::before_mint(contract, amount, account_id);
    }

    fn after_mint(contract: &mut C, amount: u128, account_id: &AccountId, _: ()) {
        SimpleNep141Hook::after_mint(contract, amount, account_id);
    }

    fn before_transfer(contract: &C, transfer: &Nep141Transfer) {
        SimpleNep141Hook::before_transfer(contract, transfer);
    }

    fn after_transfer(contract: &mut C, transfer: &Nep141Transfer, _: ()) {
        SimpleNep141Hook::after_transfer(contract, transfer);
    }

    fn before_burn(contract: &C, amount: u128, account_id: &AccountId) {
        SimpleNep141Hook::before_burn(contract, amount, account_id);
    }

    fn after_burn(contract: &mut C, amount: u128, account_id: &AccountId, _: ()) {
        SimpleNep141Hook::after_burn(contract, amount, account_id);
    }
}
