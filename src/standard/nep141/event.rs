//! NEP-141 standard events for minting, burning, and transferring tokens.

use std::borrow::Cow;

use near_sdk::{
    AccountIdRef,
    json_types::U128,
    serde::{Deserialize, Serialize},
};

use near_sdk_contract_tools_macros::event;

/// NEP-141 standard events for minting, burning, and transferring tokens.
#[event(
    crate = "crate",
    macros = "crate",
    standard = "nep141",
    version = "1.0.0"
)]
#[derive(Debug, Clone)]
pub enum Nep141Event<'a> {
    /// Token mint event. Emitted when tokens are created and total_supply is
    /// increased.
    FtMint(Vec<FtMintData<'a>>),

    /// Token transfer event. Emitted when tokens are transferred between two
    /// accounts. No change to total_supply.
    FtTransfer(Vec<FtTransferData<'a>>),

    /// Token burn event. Emitted when tokens are burned (removed from supply).
    /// Decrease in total_supply.
    FtBurn(Vec<FtBurnData<'a>>),
}

/// Individual mint metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FtMintData<'a> {
    /// Address to which new tokens were minted
    pub owner_id: Cow<'a, AccountIdRef>,
    /// Amount of minted tokens
    pub amount: U128,
    /// Optional note
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

/// Individual transfer metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FtTransferData<'a> {
    /// Account ID of the sender
    pub old_owner_id: Cow<'a, AccountIdRef>,
    /// Account ID of the receiver
    pub new_owner_id: Cow<'a, AccountIdRef>,
    /// Amount of transferred tokens
    pub amount: U128,
    /// Optional note
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<Cow<'a, str>>,
}

/// Individual burn metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct FtBurnData<'a> {
    /// Account ID from which tokens were burned
    pub owner_id: Cow<'a, AccountIdRef>,
    /// Amount of burned tokens
    pub amount: U128,
    /// Optional note
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
            Nep141Event::FtMint(vec![FtMintData {
                owner_id: AccountIdRef::new_or_panic("foundation.near").into(),
                amount: 500u128.into(),
                memo: None,
            }])
            .to_event_string(),
            r#"EVENT_JSON:{"standard":"nep141","version":"1.0.0","event":"ft_mint","data":[{"owner_id":"foundation.near","amount":"500"}]}"#,
        );
    }

    #[test]
    fn transfer() {
        assert_eq!(
            Nep141Event::FtTransfer(vec![
                FtTransferData {
                    old_owner_id: AccountIdRef::new_or_panic("from.near").into(),
                    new_owner_id: AccountIdRef::new_or_panic("to.near").into(),
                    amount: 42u128.into(),
                    memo: Some("hi hello bonjour".into()),
                },
                FtTransferData {
                    old_owner_id: AccountIdRef::new_or_panic("user1.near").into(),
                    new_owner_id: AccountIdRef::new_or_panic("user2.near").into(),
                    amount: 7500u128.into(),
                    memo: None,
                },
            ])
            .to_event_string(),
            r#"EVENT_JSON:{"standard":"nep141","version":"1.0.0","event":"ft_transfer","data":[{"old_owner_id":"from.near","new_owner_id":"to.near","amount":"42","memo":"hi hello bonjour"},{"old_owner_id":"user1.near","new_owner_id":"user2.near","amount":"7500"}]}"#,
        );
    }

    #[test]
    fn burn() {
        assert_eq!(
            Nep141Event::FtBurn(vec![FtBurnData {
                owner_id: AccountIdRef::new_or_panic("foundation.near").into(),
                amount: 100u128.into(),
                memo: None,
            }])
            .to_event_string(),
            r#"EVENT_JSON:{"standard":"nep141","version":"1.0.0","event":"ft_burn","data":[{"owner_id":"foundation.near","amount":"100"}]}"#,
        );
    }
}
