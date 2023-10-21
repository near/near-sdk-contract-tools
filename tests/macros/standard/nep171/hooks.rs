use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    log, near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::{hook::Hook, nft::*};

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Nep171)]
#[nep171(transfer_hook = "Self")]
#[near_bindgen]
pub struct Contract {
    transfer_count: u32,
}

impl Hook<Contract, Nep171Transfer<'_>> for Contract {
    type State = ();

    fn before(_contract: &Contract, transfer: &Nep171Transfer<'_>, _: &mut ()) {
        log!(
            "{:?} is transferring {} to {}",
            transfer.sender_id,
            transfer.token_id,
            transfer.receiver_id,
        );
    }

    fn after(contract: &mut Contract, _transfer: &Nep171Transfer<'_>, _: ()) {
        contract.transfer_count += 1;
    }
}
