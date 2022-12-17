//! Payroll system manages employee and their pay
use near_contract_tools::{
    approval::{self, ApprovalConfiguration, ApprovalManager},
    rbac::Rbac,
    Rbac,
};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedMap,
    env, ext_contract, near_bindgen,
    serde::Serialize,
    AccountId, BorshStorageKey, PanicOnDefault, Promise,
};

#[derive(BorshStorageKey, BorshSerialize)]
enum PayrollKey {
    LOG,
}

#[derive(BorshSerialize, BorshDeserialize)]
struct PayrollAction(AccountId, u8);

impl approval::Action<Payroll> for PayrollAction {
    type Output = Promise;

    fn execute(self, contract: &mut Payroll) -> Self::Output {
        let PayrollAction(employee_id, hours) = self;
        Promise::new(employee_id).transfer(hours as u128 * contract.hourly_fee as u128)
    }
}

/// Both manager and employee need to approve payment request
#[derive(BorshSerialize, BorshDeserialize, Serialize)]
pub enum PayrollApproval {
    EmployeeApproved,
    ManagerApproved,
    BothApproved,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize)]
struct PayApprovalConfiguration;

impl ApprovalConfiguration<PayrollAction, PayrollApproval> for PayApprovalConfiguration {
    type ApprovalError = String;
    type RemovalError = ();
    type AuthorizationError = String;
    type ExecutionEligibilityError = String;

    fn is_approved_for_execution(
        &self,
        action_request: &approval::ActionRequest<PayrollAction, PayrollApproval>,
    ) -> Result<(), Self::ExecutionEligibilityError> {
        match action_request.approval_state {
            PayrollApproval::EmployeeApproved => Err("Manager has not approved yet".to_string()),
            PayrollApproval::ManagerApproved => Err("Employee has not accepted yet".to_string()),
            PayrollApproval::BothApproved => Ok(()),
        }
    }

    fn is_removable(
        &self,
        _action_request: &approval::ActionRequest<PayrollAction, PayrollApproval>,
    ) -> Result<(), Self::RemovalError> {
        Ok(())
    }

    fn is_account_authorized(
        &self,
        account_id: &AccountId,
        action_request: &approval::ActionRequest<PayrollAction, PayrollApproval>,
    ) -> Result<(), Self::AuthorizationError> {
        match (
            <Payroll as Rbac>::has_role(account_id, &Role::Manager),
            action_request.action.0.eq(account_id),
        ) {
            (true, true) => Err("An employee cannot be their own manager".to_string()),
            (true, false) => Ok(()),
            (false, true) => Ok(()),
            (false, false) => Err("Unauthorized account".to_string()),
        }
    }

    fn try_approve_with_authorized_account(
        &self,
        account_id: AccountId,
        action_request: &mut approval::ActionRequest<PayrollAction, PayrollApproval>,
    ) -> Result<(), Self::ApprovalError> {
        match (
            <Payroll as Rbac>::has_role(&account_id, &Role::Manager),
            action_request.action.0 == account_id,
        ) {
            (true, true) => Err("An employee cannot be their own manager".to_string()),
            (true, false) => {
                match action_request.approval_state {
                    PayrollApproval::EmployeeApproved => {
                        action_request.approval_state = PayrollApproval::BothApproved
                    }
                    _ => (),
                }
                Ok(())
            }
            (false, true) => {
                match action_request.approval_state {
                    PayrollApproval::ManagerApproved => {
                        action_request.approval_state = PayrollApproval::BothApproved
                    }
                    _ => (),
                }
                Ok(())
            }
            (false, false) => Err("Unauthorized account".to_string()),
        }
    }
}

impl ApprovalManager<PayrollAction, PayrollApproval, PayApprovalConfiguration> for Payroll {}

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
///
/// Note: After any &mut self function the contract state is automatically
/// written back. An explicit env::state_write call is not needed.
///
/// https://docs.near.org/sdk/rust/contract-structure/near-bindgen
#[near_bindgen]
impl Payroll {
    #[init]
    pub fn new(owner: &AccountId) -> Self {
        let mut contract = Payroll {
            hourly_fee: 1000,
            logged_time: UnorderedMap::new(PayrollKey::LOG),
        };
        <Payroll as ApprovalManager<PayrollAction, PayrollApproval, PayApprovalConfiguration>>::init(PayApprovalConfiguration {});
        contract.add_role(owner, &Role::Manager);
        contract
    }

    /// Manager can add new managers
    pub fn add_manager(&mut self, account_id: &AccountId) {
        self.require_role(&Role::Manager);
        self.add_role(account_id, &Role::Manager);
    }

    /// Manager can add new employees
    pub fn add_employee(&mut self, account_id: &AccountId) {
        self.require_role(&Role::Manager);
        self.add_role(account_id, &Role::Employee);
        self.logged_time.insert(account_id, &0);
    }

    pub fn approve_pay(&mut self, request_id: u32) {
        self.approve_request(request_id).unwrap();
    }

    pub fn get_pay(&mut self, request_id: u32) -> Promise {
        self.execute_request(request_id).unwrap()
    }

    /// Employee can request pay
    pub fn request_pay(&mut self) -> u32 {
        self.require_role(&Role::Employee);
        let employee_id = env::predecessor_account_id();
        let logged_time = self.logged_time.get(&employee_id).unwrap_or_else(|| {
            env::panic_str(format!("No record exists for account: {}", employee_id).as_str())
        });

        let request_id = self
            .create_request(
                PayrollAction(employee_id, logged_time),
                PayrollApproval::EmployeeApproved,
            )
            .unwrap();

        near_sdk::log!(format!("Request ID: {request_id}"));

        request_id
    }

    /// Employee can log time
    pub fn log_time(&mut self, hours: u8) {
        self.require_role(&Role::Employee);
        let employee_id = env::predecessor_account_id();
        let current_hours = self.logged_time.get(&employee_id).unwrap_or_else(|| {
            env::panic_str(format!("No record exists for account: {}", employee_id).as_str())
        });

        // Add entry for employee's account id
        self.logged_time
            .insert(&employee_id, &(current_hours + hours));
    }

    /// Employee can check the time they've logged
    pub fn get_logged_time(&self) -> u8 {
        self.require_role(&Role::Employee);
        let employee_id = env::predecessor_account_id();

        self.logged_time.get(&employee_id).unwrap_or_else(|| {
            env::panic_str(format!("No record exists for account: {}", employee_id).as_str())
        })
    }

    pub fn is_employee(&self) -> bool {
        self.require_role(&Role::Employee);
        true
    }
}

#[ext_contract(ext_payroll)]
/// External methods for payroll
///
/// TODO: what does externally accessible functions mean
/// if in tests different accounts can call the internal
/// functions as in workspace_tests/tests/payroll.rs
pub trait PayrollExternal {
    /// Manager can add new managers
    fn payroll_add_manager(&mut self, account_id: &AccountId);
    /// Manager can add new employees
    fn payroll_add_employee(&mut self, account_id: AccountId);
    /// Employee can log time
    fn payroll_log_time(&mut self, hours: u8);
    /// Employee can request for pay
    fn payroll_request_pay(&mut self) -> u32;
    /// Pay can be approved by manager and the employee who made the request
    fn payroll_approve_pay(&mut self, request_id: u32);
    /// If request is approved get_pay will return a promise to transfer funds
    fn payroll_get_pay(&mut self, request_id: u32) -> Promise;
}

impl PayrollExternal for Payroll {
    fn payroll_add_manager(&mut self, account_id: &AccountId) {
        self.add_manager(account_id);
    }

    fn payroll_add_employee(&mut self, account_id: AccountId) {
        self.add_employee(&account_id);
    }

    fn payroll_log_time(&mut self, hours: u8) {
        self.log_time(hours);
    }

    fn payroll_request_pay(&mut self) -> u32 {
        self.request_pay()
    }

    fn payroll_approve_pay(&mut self, request_id: u32) {
        self.approve_pay(request_id);
    }

    fn payroll_get_pay(&mut self, request_id: u32) -> Promise {
        self.get_pay(request_id)
    }
}

pub fn main() {}

#[cfg(test)]
mod tests {
    use near_contract_tools::rbac::Rbac;
    use near_sdk::{test_utils::VMContextBuilder, testing_env, AccountId};

    use crate::{Payroll, Role};

    #[test]
    fn test_owner() {
        let owner: AccountId = "account_owner".parse().unwrap();
        let _contract = Payroll::new(&owner);
        assert!(Payroll::has_role(&owner, &Role::Manager));
    }

    #[test]
    fn test_add_manager() {
        let owner: AccountId = "account_owner".parse().unwrap();
        let manager: AccountId = "account_manager".parse().unwrap();

        let mut contract = Payroll::new(&owner);

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner.clone())
            .build());

        contract.add_manager(&manager);

        assert!(Payroll::has_role(&owner, &Role::Manager));
        assert!(Payroll::has_role(&manager, &Role::Manager));
    }

    #[test]
    fn test_add_employees() {
        let owner: AccountId = "account_owner".parse().unwrap();
        let manager: AccountId = "account_manager".parse().unwrap();
        let emp1: AccountId = "account_emp1".parse().unwrap();
        let emp2: AccountId = "account_emp2".parse().unwrap();

        let mut contract = Payroll::new(&owner);

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner.clone())
            .build());

        contract.add_manager(&manager);

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(manager.clone())
            .build());

        contract.add_employee(&emp1);
        contract.add_employee(&emp2);

        assert!(Payroll::has_role(&emp1, &Role::Employee));
        assert!(Payroll::has_role(&emp2, &Role::Employee));
    }

    #[test]
    fn test_employee_log_time() {
        let owner: AccountId = "account_owner".parse().unwrap();
        let emp1: AccountId = "account_emp1".parse().unwrap();
        let emp2: AccountId = "account_emp2".parse().unwrap();

        let mut contract = Payroll::new(&owner);

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner.clone())
            .build());

        contract.add_employee(&emp1);
        contract.add_employee(&emp2);

        assert!(Payroll::has_role(&emp1, &Role::Employee));
        assert_eq!(contract.logged_time.get(&emp1), Some(0));

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(emp1.clone())
            .build());
        contract.log_time(10);
        assert_eq!(contract.logged_time.get(&emp1), Some(10));

        // time is incremented correctly
        contract.log_time(10);
        assert_eq!(contract.logged_time.get(&emp1), Some(20));

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(emp2.clone())
            .build());

        // time is incremented correctly for the user who made the call
        contract.log_time(9);
        assert_eq!(contract.logged_time.get(&emp1), Some(20));
        assert_eq!(contract.logged_time.get(&emp2), Some(9));

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(emp1.clone())
            .build());
        assert_eq!(contract.get_logged_time(), 20);

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(emp2.clone())
            .build());
        assert_eq!(contract.get_logged_time(), 9);

        // logged_time will panic if it is called by a user with manager role
        // this cannot be tested because panic unwind will need non-mutable
        // reference to contract
    }
}
