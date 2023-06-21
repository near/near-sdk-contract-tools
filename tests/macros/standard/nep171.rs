// use ;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk_contract_tools::slot::Slot;

// #[derive(Nep171)]
#[derive(BorshDeserialize, BorshSerialize)]
#[near_sdk::near_bindgen]
struct NonFungibleToken {}

impl near_sdk_contract_tools::standard::nep171::Nep171ControllerInternal for NonFungibleToken {
    fn root() -> Slot<()> {
        Slot::root(b"nft" as &[u8])
    }
}

impl near_sdk_contract_tools::standard::nep171::Nep171 for NonFungibleToken {
    fn nft_transfer(
        &mut self,
        receiver_id: near_sdk::AccountId,
        token_id: String,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) {
        use near_sdk_contract_tools::standard::nep171::*;

        near_sdk::require!(
            approval_id.is_none(),
            APPROVAL_MANAGEMENT_NOT_SUPPORTED_MESSAGE,
        );

        near_sdk::assert_one_yocto();

        let sender_id = near_sdk::env::predecessor_account_id();

        Nep171Controller::transfer(self, token_id, sender_id, receiver_id, memo).unwrap();
    }

    fn nft_transfer_call(
        &mut self,
        receiver_id: near_sdk::AccountId,
        token_id: String,
        approval_id: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> near_sdk::PromiseOrValue<bool> {
        use near_sdk_contract_tools::standard::nep171::*;

        near_sdk::require!(
            approval_id.is_none(),
            APPROVAL_MANAGEMENT_NOT_SUPPORTED_MESSAGE
        );

        near_sdk::assert_one_yocto();

        near_sdk::require!(
            near_sdk::env::prepaid_gas() > GAS_FOR_NFT_TRANSFER_CALL,
            INSUFFICIENT_GAS_MESSAGE,
        );

        let sender_id = near_sdk::env::predecessor_account_id();

        Nep171Controller::transfer(
            self,
            token_id.clone(),
            sender_id.clone(),
            receiver_id.clone(),
            memo,
        )
        .unwrap();

        ext_nep171_receiver::ext(receiver_id.clone())
            .with_static_gas(near_sdk::env::prepaid_gas() - GAS_FOR_NFT_TRANSFER_CALL)
            .nft_on_transfer(
                sender_id.clone(),
                receiver_id.clone(),
                token_id.clone(),
                msg,
            )
            .then(
                ext_nep171::ext(near_sdk::env::current_account_id())
                    .with_static_gas(GAS_FOR_RESOLVE_TRANSFER)
                    .nft_resolve_transfer(sender_id, receiver_id, token_id, None),
            )
            .into()
    }

    fn nft_token(
        &self,
        token_id: String,
    ) -> Option<near_sdk_contract_tools::standard::nep171::Token> {
        use near_sdk_contract_tools::standard::nep171::*;

        Nep171Controller::token_owner(self, token_id.clone()).map(|owner_id| Token {
            token_id,
            owner_id,
        })
    }

    fn nft_resolve_transfer(
        previous_owner_id: near_sdk::AccountId,
        receiver_id: near_sdk::AccountId,
        token_id: String,
        approved_account_ids: Option<std::collections::HashMap<String, u64>>,
    ) -> bool {
        todo!()
    }
}
