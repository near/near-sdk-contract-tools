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
///     #[event = "nft_mint"]
///     NftMint(Vec<Nep171NftMintData>),
/// }
/// ```
pub trait Event {
    /// Consumes the event and returns an `EVENT_JSON:{}`-formatted log string
    fn into_event_string(self) -> String;
    /// Consumes the event and emits it to the NEAR blockchain
    fn emit(self);
}

pub trait EventMetadata {
    fn standard(&self) -> String;
    fn version(&self) -> String;
    fn event(&self) -> String;
}

/// NEP-297 Event Log Data
/// https://github.com/near/NEPs/blob/master/neps/nep-0297.md#specification
#[derive(Serialize)]
struct EventLogData<T> {
    pub standard: String,
    pub version: String,
    pub event: String,
    pub data: T,
}

impl<T: EventMetadata> From<T> for EventLogData<T> {
    fn from(m: T) -> Self {
        Self {
            standard: m.standard(),
            version: m.version(),
            event: m.event(),
            data: m,
        }
    }
}

impl<T: Serialize + EventMetadata> Event for T {
    fn into_event_string(self) -> String {
        format!(
            "EVENT_JSON:{}",
            serde_json::to_string(&Into::<EventLogData<_>>::into(self))
                .unwrap_or_else(|_| near_sdk::env::abort()),
        )
    }

    fn emit(self) {
        near_sdk::env::log_str(&self.into_event_string());
    }
}
