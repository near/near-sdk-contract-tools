//! Hooks to integrate NEP-171 with other components.

use crate::{
    hook::Hook,
    standard::{nep145::Nep145ForceUnregister, nep181::Nep181Controller},
};

use super::{action::Nep171Burn, Nep171Controller};

/// Hook that burns all NEP-171 tokens held by an account when the account
/// performs an NEP-145 force unregister.
pub struct BurnNep171OnForceUnregisterHook;

impl<C> Hook<C, Nep145ForceUnregister<'_>> for BurnNep171OnForceUnregisterHook
where
    C: Nep171Controller + Nep181Controller,
{
    fn hook<R>(
        contract: &mut C,
        action: &Nep145ForceUnregister<'_>,
        f: impl FnOnce(&mut C) -> R,
    ) -> R {
        let token_ids = contract.with_tokens_for_owner(action.account_id, |t| {
            t.iter().collect::<Vec<_>>()
        });

        contract
            .burn(&Nep171Burn {
                token_ids: &token_ids,
                owner_id: action.account_id,
                memo: Some("storage forced unregistration"),
            })
            .unwrap_or_else(|e| {
                near_sdk::env::panic_str(&format!(
                    "Failed to burn tokens during forced unregistration: {e}",
                ))
            });

        f(contract)
    }
}
