workspaces_tests::predicate!();

use near_sdk::{AccountId, PanicOnDefault, env, near};
use near_sdk_contract_tools::{Escrow, escrow::Escrow};

#[derive(Clone)]
#[near(serializers = [borsh, json])]
pub enum PrimaryColour {
    Red,
    Yellow,
    Blue,
}

#[derive(Clone)]
#[near(serializers = [borsh, json])]
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

#[derive(Escrow, PanicOnDefault)]
#[escrow(id = "PrimaryColour", state = "AccountId")]
#[near(contract_state)]
pub struct Contract {}

#[near]
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
