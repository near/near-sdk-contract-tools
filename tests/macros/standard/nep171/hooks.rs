use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    log, near_bindgen, PanicOnDefault,
};
use near_sdk_contract_tools::nft::*;

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Nep171)]
#[near_bindgen]
pub struct Contract {
    transfer_count: u32,
}

impl SimpleNep171Hook for Contract {
    fn before_nft_transfer(&self, transfer: &Nep171Transfer) {
        log!(
            "{:?} is transferring {} to {}",
            transfer.sender_id,
            transfer.token_id,
            transfer.receiver_id,
        );
    }

    fn after_nft_transfer(&mut self, _transfer: &Nep171Transfer) {
        self.transfer_count += 1;
    }
}
