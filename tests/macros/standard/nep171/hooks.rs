use near_sdk_contract_tools::{compat_derive_borsh, compat_use_borsh, hook::Hook, nft::*};
compat_use_borsh!();
use near_sdk::{log, near_bindgen, PanicOnDefault};

compat_derive_borsh! {
    #[derive(PanicOnDefault, Nep171)]
    #[nep171(transfer_hook = "Self")]
    #[near_bindgen]
    pub struct Contract {
        transfer_count: u32,
    }
}

impl Hook<Contract, Nep171Transfer<'_>> for Contract {
    fn hook<R>(
        contract: &mut Contract,
        args: &Nep171Transfer<'_>,
        f: impl FnOnce(&mut Contract) -> R,
    ) -> R {
        log!(
            "{:?} is transferring {} to {}",
            args.sender_id,
            args.token_id,
            args.receiver_id,
        );
        let r = f(contract);
        contract.transfer_count += 1;
        r
    }
}
