//! Pause pattern implements methods to pause, unpause and get the status of the
//! contract.
//!
//! [`Pause`] implements methods to pause and unpause the contract. When the
//! methods are called the contracts status changes and the respective event
//! is emitted. A contract starts off as "unpaused" by default. [`PauseExternal`]
//! exposes an external function to check the status of the contract.
//!
//! This [derive macro](near_sdk_contract_tools_macros::Pause)
//! derives a default implementation for both these traits.
//!
//! # Safety
//! The default implementation assumes or enforces the following invariants.
//! Violating assumed invariants may corrupt contract state and show unexpected
//! behavior (UB). Enforced invariants throw an error (ERR) but contract
//! state remains intact.
//!
//! * Initial state is unpaused.
//! * (UB) The pause root storage slot is not used or modified. The default key is `~p`.
//! * (ERR) Only an "unpaused" contract can call `pause`.
//! * (ERR) Only a "paused" contract can call `unpause`.
//! * (ERR) [`Pause::require_paused`] may only be called when the contract is paused.
//! * (ERR) [`Pause::require_unpaused`] may only be called when the contract is unpaused.
#![allow(missing_docs)] // #[ext_contract(...)] does not play nicely with clippy

use crate::{slot::Slot, standard::nep297::Event, DefaultStorageKey};
use near_sdk::{ext_contract, require};
use near_sdk_contract_tools_macros::event;

const UNPAUSED_FAIL_MESSAGE: &str = "Disallowed while contract is unpaused";
const PAUSED_FAIL_MESSAGE: &str = "Disallowed while contract is paused";

/// Events emitted when contract pause state is changed
#[event(
    standard = "x-paus",
    version = "1.0.0",
    crate = "crate",
    macros = "near_sdk_contract_tools_macros"
)]
#[derive(Debug, Clone)]
pub enum PauseEvent {
    /// Emitted when the contract is paused
    Pause,
    /// Emitted when the contract is unpaused
    Unpause,
}

/// Internal functions for [`Pause`]. Using these methods may result in unexpected behavior.
pub trait PauseInternal {
    /// Storage root
    fn root() -> Slot<()> {
        Slot::new(DefaultStorageKey::Pause)
    }

    /// Storage slot for pause state
    fn slot_paused() -> Slot<bool> {
        Self::root().transmute()
    }
}

/// Contract private-only interactions for a pausable contracts.
///
/// # Examples
///
/// ```
/// use near_sdk::near_bindgen;
/// use near_sdk_contract_tools::{pause::Pause, Pause};
///
/// #[derive(Pause)]
/// #[near_bindgen]
/// struct Contract {
///     // ...
/// }
///
/// #[near_bindgen]
/// impl Contract {
///     pub fn only_when_unpaused(&self) {
///         Self::require_unpaused();
///     }
///
///     pub fn only_when_paused(&self) {
///         Self::require_paused();
///     }
///
///     pub fn emergency_shutdown(&mut self) {
///         self.pause();
///     }
///
///     pub fn emergency_shutdown_end(&mut self) {
///         self.unpause();
///     }
/// }
/// ```
pub trait Pause {
    /// Force the contract pause state in a particular direction.
    /// Does not emit events or check the current pause state.
    fn set_is_paused(&mut self, is_paused: bool);

    /// Returns `true` if the contract is paused, `false` otherwise
    fn is_paused() -> bool;

    /// Pauses the contract if it is currently unpaused, panics otherwise.
    /// Emits a `PauseEvent::Pause` event.
    fn pause(&mut self);

    /// Unpauses the contract if it is currently paused, panics otherwise.
    /// Emits a `PauseEvent::Unpause` event.
    fn unpause(&mut self);

    /// Rejects if the contract is unpaused.
    fn require_paused();

    /// Rejects if the contract is paused.
    fn require_unpaused();
}

impl<T: PauseInternal> Pause for T {
    fn set_is_paused(&mut self, is_paused: bool) {
        Self::slot_paused().write(&is_paused);
    }

    fn is_paused() -> bool {
        Self::slot_paused().read().unwrap_or(false)
    }

    fn pause(&mut self) {
        Self::require_unpaused();
        self.set_is_paused(true);
        PauseEvent::Pause.emit();
    }

    fn unpause(&mut self) {
        Self::require_paused();
        self.set_is_paused(false);
        PauseEvent::Unpause.emit();
    }

    fn require_paused() {
        require!(Self::is_paused(), UNPAUSED_FAIL_MESSAGE);
    }

    fn require_unpaused() {
        require!(!Self::is_paused(), PAUSED_FAIL_MESSAGE);
    }
}

/// External (public) methods for [`Pause`]
#[ext_contract(ext_pause)]
pub trait PauseExternal {
    /// Returns `true` if the contract is paused, `false` otherwise
    fn paus_is_paused(&self) -> bool;
}
