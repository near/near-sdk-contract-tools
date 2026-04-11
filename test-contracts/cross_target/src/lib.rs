use near_sdk::{AccountId, PanicOnDefault, env, near, require};

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct Contract {
    owner_id: AccountId,
    value: String,
    calls: u32,
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            value: "".into(),
            calls: 0,
        }
    }

    pub fn set_value(&mut self, value: String) {
        require!(env::predecessor_account_id() == self.owner_id, "Owner only");
        self.value = value;
        self.calls += 1;
    }

    pub fn get_value(&self) -> &str {
        &self.value
    }

    pub fn get_calls(&self) -> u32 {
        self.calls
    }
}
