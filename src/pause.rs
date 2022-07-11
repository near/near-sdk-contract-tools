//! Contract method pausing/unpausing

use crate::{event::Event, near_contract_tools, slot::Slot};
use near_contract_tools_macros::Event;
use near_sdk::require;
use serde::Serialize;

/// Events emitted when contract pause state is changed
#[derive(Event, Serialize)]
#[event(standard = "x-paus", version = "1.0.0", rename_all = "snake_case")]
#[serde(untagged)]
pub enum PauseEvent {
    /// Emitted when the contract is paused
    Pause,
    /// Emitted when the contract is unpaused
    Unpause,
}

/// Internal-only interactions for a pausable contract
///
/// # Examples
///
/// ```
/// use near_sdk::near_bindgen;
/// use near_contract_tools::{pause::Pause, Pause};
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
///         self.require_unpaused();
///     }
///
///     pub fn only_when_paused(&self) {
///         self.require_paused();
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
    /// Storage root
    fn root(&self) -> Slot<()>;

    /// Storage slot for pause state
    fn slot_paused(&self) -> Slot<bool> {
        unsafe { self.root().transmute() }
    }

    /// Force the contract pause state in a particular direction.
    /// Does not emit events or check the current pause state.
    fn set_is_paused(&mut self, is_paused: bool) {
        self.slot_paused().write(&is_paused);
    }

    /// Returns `true` if the contract is paused, `false` otherwise
    fn is_paused(&self) -> bool {
        self.slot_paused().read().unwrap_or(false)
    }

    /// Pauses the contract if it is currently unpaused, panics otherwise.
    /// Emits a `PauseEvent::Pause` event.
    fn pause(&mut self) {
        self.require_unpaused();
        self.set_is_paused(true);
        PauseEvent::Pause.emit();
    }

    /// Unpauses the contract if it is currently paused, panics otherwise.
    /// Emits a `PauseEvent::Unpause` event.
    fn unpause(&mut self) {
        self.require_paused();
        self.set_is_paused(false);
        PauseEvent::Unpause.emit();
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

/// External methods for `Pause`
pub trait PauseExternal {
    /// Returns `true` if the contract is paused, `false` otherwise
    fn paus_is_paused(&self) -> bool;
}
