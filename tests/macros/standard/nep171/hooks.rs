use near_sdk::{PanicOnDefault, log, near};
use near_sdk_contract_tools::{hook::Hook, nft::*};

#[derive(Nep171, PanicOnDefault)]
#[nep171(transfer_hook = "Self")]
#[near(contract_state)]
pub struct Contract {
    transfer_count: u32,
}

impl Hook<Contract, Nep171Transfer<'_>> for Contract {
    fn hook<R>(
        contract: &mut Contract,
        args: &Nep171Transfer,
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
