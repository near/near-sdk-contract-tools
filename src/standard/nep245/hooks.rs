//! Hooks to integrate NEP-245 with other standards.

use crate::{hook::Hook, standard::nep145::Nep145ForceUnregister};

use super::{Nep245Burn, Nep245Controller, Nep245ControllerInternal};

/// Hook that burns all tokens on NEP-145 force unregister.
pub struct BurnNep245OnForceUnregisterHook;

impl<C: Nep245Controller + Nep245ControllerInternal> Hook<C, Nep145ForceUnregister<'_>>
    for BurnNep245OnForceUnregisterHook
{
    fn hook<R>(
        contract: &mut C,
        args: &Nep145ForceUnregister<'_>,
        f: impl FnOnce(&mut C) -> R,
    ) -> R {
        let r = f(contract);

        let tokens = contract.tokens();
        #[allow(clippy::cast_possible_truncation)]
        let mut burn = Nep245Burn::empty(tokens.len() as usize, args.account_id.clone())
            .memo("storage forced unregistration");

        for token_id in tokens.iter() {
            burn = burn.and_burn(
                token_id.clone(),
                contract.balance_of(&token_id, &args.account_id),
            );
        }

        contract.burn(&burn).unwrap_or_else(|e| {
            near_sdk::env::panic_str(&format!(
                "Failed to burn tokens during forced unregistration: {e}",
            ))
        });

        for token_id in tokens.iter() {
            <C as Nep245ControllerInternal>::slot_balance(&token_id, &args.account_id).remove();
        }

        r
    }
}
