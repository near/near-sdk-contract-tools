//! Hooks to integrate NEP-141 with other standards.

use crate::{hook::Hook, standard::nep145::Nep145ForceUnregister};

use super::{Nep141Burn, Nep141Controller, Nep141ControllerInternal};

/// Hook that burns all tokens on NEP-145 force unregister.
pub struct BurnOnForceUnregisterHook;

impl<C: Nep141Controller + Nep141ControllerInternal> Hook<C, Nep145ForceUnregister<'_>>
    for BurnOnForceUnregisterHook
{
    fn hook<R>(
        contract: &mut C,
        args: &Nep145ForceUnregister<'_>,
        f: impl FnOnce(&mut C) -> R,
    ) -> R {
        let r = f(contract);

        let balance = contract.balance_of(args.account_id);
        contract
            .burn(&Nep141Burn {
                amount: balance,
                account_id: args.account_id,
                memo: Some("storage forced unregistration"),
            })
            .unwrap_or_else(|e| {
                near_sdk::env::panic_str(&format!(
                    "Failed to burn tokens during forced unregistration: {e}",
                ))
            });

        <C as Nep141ControllerInternal>::slot_account(args.account_id).remove();

        r
    }
}
