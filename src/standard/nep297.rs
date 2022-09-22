//! Helpers for `#[derive(near_contract_tools::Nep297)]`

use near_sdk::serde::Serialize;

/// Emit events according to the [NEP-297 event standard](https://nomicon.io/Standards/EventsFormat).
///
/// # Examples
///
/// ## Normal events
///
/// ```
/// use near_contract_tools::event;
///
/// #[event(standard = "nft", version = "1.0.0")]
/// pub struct MintEvent {
///     pub owner_id: String,
///     pub token_id: String,
/// }
///
/// let e = MintEvent {
///     owner_id: "account".to_string(),
///     token_id: "token_1".to_string(),
/// };
///
/// use near_contract_tools::standard::nep297::Event;
///
/// e.emit();
/// ```
pub trait Event<T: ?Sized> {
    /// Retrieves the event log before serialization
    fn event_log(&self) -> EventLog<&T>;

    /// Converts the event into an NEP-297 event-formatted string
    fn to_event_string(&self) -> String
    where
        T: Serialize,
    {
        format!(
            "EVENT_JSON:{}",
            serde_json::to_string(&self.event_log()).unwrap_or_else(|_| near_sdk::env::abort()),
        )
    }

    /// Emits the event string to the blockchain
    fn emit(&self)
    where
        T: Serialize,
    {
        near_sdk::env::log_str(&self.to_event_string());
    }
}

/// NEP-297 Event Log Data
/// <https://github.com/near/NEPs/blob/master/neps/nep-0297.md#specification>
#[derive(Serialize, Clone, Debug)]
pub struct EventLog<T> {
    /// Name of the event standard, e.g. "nep171"
    pub standard: &'static str,
    /// Version of the standard, e.g. "1.0.0"
    pub version: &'static str,
    /// Name of the particular event, e.g. "nft_mint", "ft_transfer"
    pub event: &'static str,
    /// Data type of the event metadata
    pub data: T,
}
