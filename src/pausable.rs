//! Contract method pausing/unpausing

use crate::{event::Event, near_contract_tools};
use near_contract_tools_macros::Event;
use near_sdk::require;
use serde::Serialize;

/// Events emitted when contract pause state is changed
#[derive(Event, Serialize)]
#[event(standard = "x-paus", version = "1.0.0", rename_all = "snake_case")]
#[serde(untagged)]
pub enum PausableEvent {
    /// Emitted when the contract is paused
    Pause,
    /// Emitted when the contract is unpaused
    Unpause,
}

/// Externally callable interface
pub trait Pausable {
    /// Returns `true` if the contract is paused, `false` otherwise
    fn paus_is_paused(&self) -> bool;
}

/// Internal-only interactions for a pausable contract
pub trait PausableController {
    /// Force the contract pause state in a particular direction.
    /// Does not emit events or check the current pause state.
    fn set_is_paused(&self, is_paused: bool);
    /// Returns `true` if the contract is paused, `false` otherwise
    fn is_paused(&self) -> bool;

    /// Pauses the contract if it is currently unpaused, panics otherwise.
    /// Emits a `PausableEvent::Pause` event.
    fn pause(&self) {
        self.require_unpaused();
        self.set_is_paused(true);
        PausableEvent::Pause.emit();
    }

    /// Unpauses the contract if it is currently paused, panics otherwise.
    /// Emits a `PausableEvent::Unpause` event.
    fn unpause(&self) {
        self.require_paused();
        self.set_is_paused(false);
        PausableEvent::Unpause.emit();
    }

    /// Rejects if the contract is unpaused
    fn require_paused(&self) {
        require!(self.is_paused(), "Disallowed while contract is unpaused");
    }

    /// Rejects if the contract is paused
    fn require_unpaused(&self) {
        require!(!self.is_paused(), "Disallowed while contract is paused");
    }
}
