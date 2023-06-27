#![allow(missing_docs)]

use std::str::FromStr;

use near_sdk_contract_tools::{rbac::Rbac, Rbac};

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    env, near_bindgen,
    serde::Serialize,
    AccountId, BorshStorageKey, PanicOnDefault,
};

pub fn main() {}

#[derive(BorshSerialize, BorshStorageKey)]
pub enum Role {
    Alpha,
    Beta,
    Gamma,
    Delta,
}

impl FromStr for Role {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "a" => Ok(Self::Alpha),
            "b" => Ok(Self::Beta),
            "g" => Ok(Self::Gamma),
            "d" => Ok(Self::Delta),
            _ => Err(()),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, PanicOnDefault, Rbac)]
#[rbac(roles = "Role")]
#[serde(crate = "near_sdk::serde")]
#[near_bindgen]
pub struct Contract {
    pub alpha: u32,
    pub beta: u32,
    pub gamma: u32,
    pub delta: u32,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            alpha: 0,
            beta: 0,
            gamma: 0,
            delta: 0,
        }
    }

    pub fn acquire_role(&mut self, role: String) {
        let role: Role = Role::from_str(&role).expect("Invalid role identifier");
        let predecessor = env::predecessor_account_id();
        self.add_role(predecessor, &role);
    }

    pub fn members(&self, role: String) -> Vec<AccountId> {
        let role: Role = Role::from_str(&role).expect("Invalid role identifier");
        Self::iter_members_of(&role).collect()
    }

    pub fn count_members(&self, role: String) -> u32 {
        let role: Role = Role::from_str(&role).expect("Invalid role identifier");
        Self::iter_members_of(&role).count() as u32
    }

    pub fn requires_alpha(&mut self) {
        Self::require_role(&Role::Alpha);
        self.alpha += 1;
    }

    pub fn requires_beta(&mut self) {
        Self::require_role(&Role::Beta);
        self.beta += 1;
    }

    pub fn requires_gamma(&mut self) {
        Self::require_role(&Role::Gamma);
        self.gamma += 1;
    }

    pub fn requires_delta(&mut self) {
        Self::require_role(&Role::Delta);
        self.delta += 1;
    }

    pub fn get(&self) -> &Self {
        self
    }
}
