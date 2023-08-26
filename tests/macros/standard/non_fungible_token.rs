use near_sdk::{
    borsh::{self, *},
    *,
};
use near_sdk_contract_tools::{standard::nep171::*, Nep171};

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Nep171)]
#[near_bindgen]
pub struct Contract {
    transfers: u32,
}

impl Nep171Hook for Contract {
    fn before_nft_transfer(_contract: &Self, transfer: &Nep171Transfer) {
        log!(
            "{} is transferring {} to {}",
            transfer.sender_id,
            transfer.token_id,
            transfer.receiver_id,
        );
    }

    fn after_nft_transfer(contract: &mut Self, _transfer: &Nep171Transfer, _: ()) {
        contract.transfers += 1;
    }
}

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Nep171)]
#[nep171(no_hooks)]
#[near_bindgen]
pub struct Contract1 {}
