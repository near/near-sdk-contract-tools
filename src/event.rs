//! Helpers for `#[derive(near_contract_tools::Event)]`

use near_sdk::serde::Serialize;

/// Emit events according to the [NEP-297 event standard](https://nomicon.io/Standards/EventsFormat).
///
/// # Examples
/// ```
/// use near_contract_tools::event::*;
/// use near_contract_tools::Event;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// pub struct Nep171NftMintData {
///     pub owner_id: String,
///     pub token_ids: Vec<String>,
/// }
///
/// #[derive(Event, Serialize)]
/// #[event(standard = "nep171", version = "1.0.0")]
/// #[serde(untagged)]
/// pub enum Nep171 {
///     #[event(name = "nft_mint")]
///     NftMint(Vec<Nep171NftMintData>),
/// }
/// ```
pub trait Event {
    /// Returns an `EVENT_JSON:{}`-formatted log string
    fn to_event_string(&self) -> String;
    /// Consumes the event and emits it to the NEAR blockchain
    fn emit(&self);
}

/// Metadata for NEP-297-compliant events & variants
pub trait EventMetadata {
    /// The name of the event standard, e.g. "nep171"
    fn standard(&self) -> &'static str;
    /// Version of the standard, e.g. "1.0.0"
    fn version(&self) -> &'static str;
    /// What type of event within the event standard, e.g. "nft_mint"
    fn event(&self) -> &'static str;
}

/// NEP-297 Event Log Data
/// <https://github.com/near/NEPs/blob/master/neps/nep-0297.md#specification>
#[derive(Serialize, Debug)]
struct EventLogData<'a, T> {
    pub standard: &'a str,
    pub version: &'a str,
    pub event: &'a str,
    pub data: &'a T,
}

impl<'a, T: EventMetadata> From<&'a T> for EventLogData<'a, T> {
    fn from(m: &'a T) -> Self {
        Self {
            standard: m.standard(),
            version: m.version(),
            event: m.event(),
            data: m,
        }
    }
}

impl<T: Serialize + EventMetadata> Event for T {
    fn to_event_string(&self) -> String {
        format!(
            "EVENT_JSON:{}",
            serde_json::to_string(&Into::<EventLogData<_>>::into(self))
                .unwrap_or_else(|_| near_sdk::env::abort()),
        )
    }

    fn emit(&self) {
        near_sdk::env::log_str(&self.to_event_string());
    }
}
