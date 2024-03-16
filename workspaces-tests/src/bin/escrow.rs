#![allow(missing_docs)]

workspaces_tests::near_sdk!();
compat_use_borsh!();
use near_sdk::{
    env, near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, PanicOnDefault,
};
use near_sdk_contract_tools::{
    compat_derive_serde_borsh, compat_use_borsh, escrow::Escrow, Escrow,
};

pub fn main() {} // Ignore

compat_derive_serde_borsh! {
    #[derive(Clone)]
    pub enum PrimaryColour {
        Red,
        Yellow,
        Blue,
    }
}

compat_derive_serde_borsh! {
    #[derive(Clone)]
    pub enum SecondaryColour {
        Orange,
        Green,
        Purple,
    }
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

compat_derive_serde_borsh! {[BorshSerialize, BorshDeserialize, Serialize],
    #[derive(PanicOnDefault, Escrow)]
    #[escrow(id = "PrimaryColour", state = "AccountId")]
    #[near_bindgen]
    pub struct Contract {}
}

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
