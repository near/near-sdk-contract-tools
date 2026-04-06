//! NEP-245 standard events for minting, burning, and transferring tokens.
//!
//! Reference: <https://github.com/near/NEPs/blob/master/neps/nep-0245/Events.md>

use std::borrow::Cow;

use near_sdk::{
    json_types::U128,
    serde::{Deserialize, Serialize},
    AccountIdRef,
};

use near_sdk_contract_tools_macros::event;

use super::TokenIdRef;

/// NEP-245 standard events for minting, burning, and transferring tokens.
#[event(
    crate = "crate",
    macros = "crate",
    standard = "nep245",
    version = "1.0.0"
)]
#[derive(Debug, Clone)]
pub enum Nep245Event<'a> {
    /// Token mint event. Emitted when tokens are created and `total_supply` is
    /// increased.
    MtMint(Vec<MtMintData<'a>>),

    /// Token transfer event. Emitted when tokens are transferred between two
    /// accounts. No change to `total_supply`.
    MtTransfer(Vec<MtTransferData<'a>>),

    /// Token burn event. Emitted when tokens are burned (removed from supply).
    /// Decrease in `total_supply`.
    MtBurn(Vec<MtBurnData<'a>>),
}

/// Individual mint metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct MtMintData<'a> {
    /// Address to which new tokens were minted.
    pub owner_id: Cow<'a, AccountIdRef>,
    /// Token IDs.
    pub token_ids: Vec<Cow<'a, TokenIdRef>>,
    /// Amounts of minted tokens.
    pub amounts: Vec<U128>,
    /// Optional note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

/// Individual transfer metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct MtTransferData<'a> {
    /// Approved account ID to transfer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<Cow<'a, AccountIdRef>>,
    /// Account ID of the sender.
    pub old_owner_id: Cow<'a, AccountIdRef>,
    /// Account ID of the receiver.
    pub new_owner_id: Cow<'a, AccountIdRef>,
    /// Token IDs.
    pub token_ids: Vec<Cow<'a, TokenIdRef>>,
    /// Amounts of transferred tokens.
    pub amounts: Vec<U128>,
    /// Optional note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

/// Individual burn metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct MtBurnData<'a> {
    /// Approved account ID to transfer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorized_id: Option<Cow<'a, AccountIdRef>>,
    /// Account ID from which tokens were burned.
    pub owner_id: Cow<'a, AccountIdRef>,
    /// Token IDs.
    pub token_ids: Vec<Cow<'a, TokenIdRef>>,
    /// Amounts of burned tokens.
    pub amounts: Vec<U128>,
    /// Optional note.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::standard::nep297::Event;

    #[test]
    fn mint() {
        assert_eq!(
            Nep245Event::MtMint(vec![MtMintData {
                owner_id: AccountIdRef::new_or_panic("foundation.near").into(),
                token_ids: vec!["aurora".into(), "proximitylabs_ft".into()],
                amounts: vec![U128(1), U128(100)],
                memo: None,
            }])
            .to_event_string(),
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_mint","data":[{"owner_id":"foundation.near","token_ids":["aurora","proximitylabs_ft"],"amounts":["1","100"]}]}"#,
        );
        assert_eq!(
            Nep245Event::MtMint(vec![
                MtMintData {
                    owner_id: AccountIdRef::new_or_panic("foundation.near").into(),
                    token_ids: vec!["aurora".into(), "proximitylabs_ft".into()],
                    amounts: vec![U128(1), U128(100)],
                    memo: None,
                },
                MtMintData {
                    owner_id: AccountIdRef::new_or_panic("user1.near").into(),
                    token_ids: vec!["meme".into()],
                    amounts: vec![U128(1)],
                    memo: None,
                }
            ])
            .to_event_string(),
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_mint","data":[{"owner_id":"foundation.near","token_ids":["aurora","proximitylabs_ft"],"amounts":["1","100"]},{"owner_id":"user1.near","token_ids":["meme"],"amounts":["1"]}]}"#
        );
    }

    #[test]
    fn transfer() {
        assert_eq!(
            Nep245Event::MtTransfer(vec![MtTransferData {
                authorized_id: None,
                old_owner_id: AccountIdRef::new_or_panic("user1.near").into(),
                new_owner_id: AccountIdRef::new_or_panic("user2.near").into(),
                token_ids: vec!["meme".into()],
                amounts: vec![U128(1)],
                memo: Some("have fun!".into()),
            },])
            .to_event_string(),
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_transfer","data":[{"old_owner_id":"user1.near","new_owner_id":"user2.near","token_ids":["meme"],"amounts":["1"],"memo":"have fun!"}]}"#
        );
        assert_eq!(
            Nep245Event::MtTransfer(vec![MtTransferData {
                authorized_id: Some(AccountIdRef::new_or_panic("thirdparty.near").into()),
                old_owner_id: AccountIdRef::new_or_panic("user2.near").into(),
                new_owner_id: AccountIdRef::new_or_panic("user3.near").into(),
                token_ids: vec!["meme".into()],
                amounts: vec![U128(1)],
                memo: Some("have fun!".into()),
            },])
            .to_event_string(),
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_transfer","data":[{"authorized_id":"thirdparty.near","old_owner_id":"user2.near","new_owner_id":"user3.near","token_ids":["meme"],"amounts":["1"],"memo":"have fun!"}]}"#
        );
    }

    #[test]
    fn burn() {
        assert_eq!(
            Nep245Event::MtBurn(vec![MtBurnData {
                authorized_id: None,
                owner_id: AccountIdRef::new_or_panic("foundation.near").into(),
                token_ids: vec!["aurora".into(), "proximitylabs_ft".into()],
                amounts: vec![U128(1), U128(100)],
                memo: None,
            }])
            .to_event_string(),
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_burn","data":[{"owner_id":"foundation.near","token_ids":["aurora","proximitylabs_ft"],"amounts":["1","100"]}]}"#
        );
        assert_eq!(
            Nep245Event::MtBurn(vec![MtBurnData {
                authorized_id: Some(AccountIdRef::new_or_panic("thirdparty.near").into()),
                owner_id: AccountIdRef::new_or_panic("foundation.near").into(),
                token_ids: vec!["aurora".into(), "proximitylabs_ft".into()],
                amounts: vec![U128(1), U128(100)],
                memo: None,
            }])
            .to_event_string(),
            r#"EVENT_JSON:{"standard":"nep245","version":"1.0.0","event":"mt_burn","data":[{"authorized_id":"thirdparty.near","owner_id":"foundation.near","token_ids":["aurora","proximitylabs_ft"],"amounts":["1","100"]}]}"#
        );
    }
}
