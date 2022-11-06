use near_contract_tools::{
    rbac::{self, Rbac},
    Rbac,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedMap,
    env, ext_contract, near_bindgen, require, AccountId, BorshStorageKey, IntoStorageKey,
    PanicOnDefault, Promise,
};

enum PayrollKey {
    LOG,
    FEE,
}

impl IntoStorageKey for PayrollKey {
    fn into_storage_key(self) -> Vec<u8> {
        match self {
            PayrollKey::LOG => b"~pl".to_vec(),
            PayrollKey::FEE => b"~pf".to_vec(),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, BorshStorageKey)]
enum Role {
    Manager,
    Employee,
}

#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault, Rbac)]
#[rbac(roles = "Role")]
#[near_bindgen]
struct Payroll {
    pub hourly_fee: u32,
    pub logged_time: UnorderedMap<AccountId, u8>,
}

/// Manager can add new employees and disburse payments
/// Employees can log time
#[near_bindgen]
impl Payroll {
    pub fn new() -> Self {
        Payroll {
            hourly_fee: 1000,
            logged_time: UnorderedMap::new(PayrollKey::LOG),
        }
    }

    /// Manager can add new employees
    pub fn add_employee(&mut self, account_id: &AccountId) {
        self.require_role(&Role::Manager);
        self.logged_time.insert(account_id, &0);

        // write updated time log to state
        env::state_write(&self.logged_time);
    }

    /// Manager can start payment
    pub fn disburse_pay(&self) {
        self.require_role(&Role::Manager);
        self.logged_time
            .iter()
            .for_each(|(account_id, logged_time)| {
                Promise::new(account_id).transfer(logged_time as u128 * self.hourly_fee as u128);
            });
    }

    /// Employee can log time
    pub fn log_time(&mut self, hours: u8) {
        self.require_role(&Role::Employee);
        let employee_id = env::predecessor_account_id();
        let current_hours = self.logged_time.get(&employee_id).unwrap_or_else(|| {
            env::panic_str(format!("No employee exists for account: {}", employee_id).as_str())
        });

        // Add entry for employee's account id
        self.logged_time
            .insert(&employee_id, &(current_hours + hours));

        // write updated time log to state
        env::state_write(&self.logged_time);
    }
}

/// External methods for payroll
#[ext_contract(ext_payroll)]
pub trait PayrollExternal {
    fn payroll_add_employee(&mut self, account_id: AccountId);
    fn disburse_pay(&self);
    fn log_time(&mut self, hours: u8);
}

impl PayrollExternal for Payroll {
    fn payroll_add_employee(&mut self, account_id: AccountId) {
        self.add_employee(&account_id);
    }

    fn disburse_pay(&self) {
        self.disburse_pay();
    }

    fn log_time(&mut self, hours: u8) {
        self.log_time(hours);
    }
}

pub fn main() {}
