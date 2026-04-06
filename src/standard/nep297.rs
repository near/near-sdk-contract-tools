//! Helpers for `#[derive(near_sdk_contract_tools::Nep297)]`

use std::borrow::Cow;

use near_sdk::{
    NearSchema,
    serde::{self, Deserialize, Serialize},
    serde_json,
};

/// Emit events according to the [NEP-297 event standard](https://nomicon.io/Standards/EventsFormat).
///
/// # Examples
///
/// ```
/// use near_sdk_contract_tools::event;
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
/// use near_sdk_contract_tools::standard::nep297::Event;
///
/// e.emit();
/// ```
pub trait Event {
    /// Converts the event into an NEP-297 event-formatted string.
    fn to_event_string(&self) -> String;

    /// Emits the event string to the blockchain.
    fn emit(&self);
}

impl<T: ToEventLog> Event for T
where
    T::Data: Serialize,
{
    fn to_event_string(&self) -> String {
        format!(
            "EVENT_JSON:{}",
            serde_json::to_string(&self.to_event_log()).unwrap_or_else(|e| {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    panic!("Failed to serialize event: {e}")
                }

                #[cfg(target_arch = "wasm32")]
                {
                    near_sdk::env::panic_str(&format!("Failed to serialize event: {e}"))
                }
            }),
        )
    }

    fn emit(&self) {
        near_sdk::env::log_str(&self.to_event_string());
    }
}

/// This type can be converted into an [`EventLog`] struct.
pub trait ToEventLog {
    /// Metadata associated with the event.
    type Data;

    /// Retrieves the event log before serialization.
    fn to_event_log(&self) -> EventLog<&Self::Data>;
}

/// NEP-297 Event Log Data
/// <https://github.com/near/NEPs/blob/master/neps/nep-0297.md#specification>
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, NearSchema)]
#[serde(crate = "near_sdk::serde")]
pub struct EventLog<'a, T> {
    /// Name of the event standard, e.g. `"nep171"`.
    pub standard: Cow<'a, str>,
    /// Version of the standard, e.g. `"1.0.0"`.
    pub version: Cow<'a, str>,
    /// Name of the particular event, e.g. `"nft_mint"`, `"ft_transfer"`.
    pub event: Cow<'a, str>,
    /// Data type of the event metadata.
    pub data: T,
}

impl<'de, T: Deserialize<'de>> EventLog<'de, T> {
    /// Deserializes an event log from a string.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the string is not a valid event log. A valid event
    /// log begins with the string `"EVENT_JSON:"`, and is followed by a JSON
    /// string.
    pub fn from_event_log_string(s: &'de str) -> Result<Self, serde::de::value::Error> {
        let data_str = s
            .strip_prefix("EVENT_JSON:")
            .ok_or(serde::de::Error::custom(serde::de::Unexpected::Str(
                "EVENT_JSON:",
            )))?;
        let data =
            serde_json::from_str::<EventLog<T>>(data_str).map_err(serde::de::Error::custom)?;
        let x = Some(1);
        x.as_ref();
        Ok(data)
    }

    /// Converts the event log into a borrowed reference.
    pub fn as_ref(&self) -> EventLog<&T> {
        EventLog {
            standard: Cow::Borrowed(&self.standard),
            version: Cow::Borrowed(&self.version),
            event: Cow::Borrowed(&self.event),
            data: &self.data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_and_from_event_log() {
        #[derive(Debug, PartialEq, Eq)]
        struct MyEvent;

        impl ToEventLog for MyEvent {
            type Data = u32;

            fn to_event_log(&self) -> EventLog<&u32> {
                EventLog {
                    standard: "nep171".into(),
                    version: "1.0.0".into(),
                    event: "nft_mint".into(),
                    data: &1,
                }
            }
        }

        let event = MyEvent;

        let string = event.to_event_string();

        assert_eq!(
            string,
            "EVENT_JSON:{\"standard\":\"nep171\",\"version\":\"1.0.0\",\"event\":\"nft_mint\",\"data\":1}"
        );

        let from_event_log_str = EventLog::<u32>::from_event_log_string(&string).unwrap();

        assert_eq!(from_event_log_str.as_ref(), event.to_event_log());
    }
}
