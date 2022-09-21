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
/// #[event(standard = "nep171", version = "1.0.0")]
/// pub struct MintEvent {
///     pub owner_id: String,
///     pub token_ids: Vec<String>,
/// }
///
/// let e = MintEvent {
///     owner_id: "account".to_string(),
///     token_ids: vec![ "t1".to_string(), "t2".to_string() ],
/// };
///
/// use near_contract_tools::standard::nep297::Event;
///
/// e.emit();
/// ```
///
/// ## Batchable events
///
/// ```
/// use near_contract_tools::{event, standard::nep297::Event};
///
/// // Note the `batch` flag
/// #[event(standard = "batch", version = "1", batch /* here */)]
/// pub struct BatchableEvent(pub &'static str);
///
/// [BatchableEvent("one"), BatchableEvent("two")].emit();
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

/// Multiple batch events can be emitted in a single log
pub trait BatchEvent {
    /// The name of the event standard, e.g. "nep171"
    fn standard() -> &'static str;
    /// Version of the standard, e.g. "1.0.0"
    fn version() -> &'static str;
    /// What type of event within the event standard, e.g. "nft_mint"
    fn event() -> &'static str;
}

impl<T: BatchEvent + Serialize, V: AsRef<[T]>> Event<[T]> for V {
    fn event_log(&self) -> EventLog<&[T]> {
        EventLog {
            standard: T::standard(),
            version: T::version(),
            event: T::event(),
            data: self.as_ref(),
        }
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

#[cfg(test)]
mod tests {
    use near_contract_tools_macros::event;

    use crate::standard::nep297::*;

    #[test]
    fn test() {
        #[event(
            standard = "my_evt",
            version = "1.0.0",
            crate = "crate",
            macros = "near_contract_tools_macros",
            batch
        )]
        struct TestEvent {
            pub foo: &'static str,
        }

        let x = TestEvent { foo: "bar" };
        let y = &[x];

        fn test_emit<T: AsRef<[TestEvent]>>(m: T) {
            m.emit();
        }

        test_emit(y);

        y.emit();

        assert_eq!(TestEvent::standard(), "my_evt");
        assert_eq!(TestEvent::version(), "1.0.0");
        assert_eq!(TestEvent::event(), "test_event");
        assert_eq!(
            y.to_event_string(),
            r#"EVENT_JSON:{"standard":"my_evt","version":"1.0.0","event":"test_event","data":[{"foo":"bar"}]}"#,
        );
    }
}
