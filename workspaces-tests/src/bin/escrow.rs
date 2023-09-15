#![allow(missing_docs)]

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, PanicOnDefault,
};
use near_sdk_contract_tools::{escrow::Escrow, Escrow};

pub fn main() {} // Ignore

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum PrimaryColour {
    Red,
    Yellow,
    Blue,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum SecondaryColour {
    Orange,
    Green,
    Purple,
}

impl From<(PrimaryColour, PrimaryColour)> for SecondaryColour {
    fn from(f: (PrimaryColour, PrimaryColour)) -> Self {
        match f {
            (PrimaryColour::Red, PrimaryColour::Yellow)
            | (PrimaryColour::Yellow, PrimaryColour::Red) => Self::Orange,
            (PrimaryColour::Blue, PrimaryColour::Yellow)
            | (PrimaryColour::Yellow, PrimaryColour::Blue) => Self::Green,
            (PrimaryColour::Red, PrimaryColour::Blue)
            | (PrimaryColour::Blue, PrimaryColour::Red) => Self::Purple,
            _ => panic!("Not a secondary colour output"),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, PanicOnDefault, Escrow)]
#[escrow(id = "PrimaryColour", state = "AccountId")]
#[serde(crate = "near_sdk::serde")]
#[near_bindgen]
pub struct Contract {}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {}
    }

    pub fn assign(&mut self, colour: PrimaryColour) {
        let predecessor = env::predecessor_account_id();
        self.lock(&colour, &predecessor);
    }

    pub fn mix(
        &mut self,
        colour: PrimaryColour,
        with: PrimaryColour,
    ) -> (AccountId, AccountId, SecondaryColour) {
        let predecessor = env::predecessor_account_id();

        let mut mixed_colour = SecondaryColour::Green;
        let mut paired = None;

        self.unlock(&with, |assignee| {
            mixed_colour = SecondaryColour::from((colour, with.clone()));
            paired = Some((predecessor, assignee.clone()));
            true
        });
        let (me, assignee) = paired.unwrap();
        (me, assignee, mixed_colour)
    }

    pub fn get_locked(&self, colour: PrimaryColour) -> bool {
        self.is_locked(&colour)
    }
}
