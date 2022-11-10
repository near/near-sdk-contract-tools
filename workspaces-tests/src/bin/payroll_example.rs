//! Payroll system manages employee and their pay
use near_contract_tools::{rbac::Rbac, Rbac};
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
        env::state_write(self);
    }

    /// Employee can request pay
    pub fn request_pay(&self) -> Promise {
        self.require_role(&Role::Employee);
        let employee_id = env::predecessor_account_id();
        let logged_time = self.logged_time.get(&employee_id).unwrap_or_else(|| {
            env::panic_str(format!("No employee exists for account: {}", employee_id).as_str())
        });
        Promise::new(employee_id).transfer(logged_time as u128 * self.hourly_fee as u128)
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
        env::state_write(self);
    }
}

#[ext_contract(ext_payroll)]
/// External methods for payroll
pub trait PayrollExternal {
    /// Manager can add new employees
    fn payroll_add_employee(&mut self, account_id: AccountId);
    /// Employee can log time
    fn payroll_log_time(&mut self, hours: u8);
    /// Employee can request for pay
    fn payroll_request_pay(&self) -> Promise;
}

impl PayrollExternal for Payroll {
    fn payroll_add_employee(&mut self, account_id: AccountId) {
        self.add_employee(&account_id);
    }

    fn payroll_log_time(&mut self, hours: u8) {
        self.log_time(hours);
    }

    fn payroll_request_pay(&self) -> Promise {
        self.request_pay()
    }
}

pub fn main() {}
