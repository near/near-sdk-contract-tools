//! Role-based access control

use near_sdk::{borsh::BorshSerialize, env, require, AccountId, IntoStorageKey};

use crate::slot::Slot;

/// Role-based access control
pub trait Rbac<R>
where
    R: BorshSerialize + IntoStorageKey,
{
    /// Storage slot namespace for items
    fn root() -> Slot<()>;

    /// Returns whether a given account has been given a certain role.
    fn has_role(account_id: &AccountId, role: &R) -> bool {
        Self::root()
            .ns(role.try_to_vec().unwrap())
            .field(account_id.try_to_vec().unwrap())
            .read()
            .unwrap_or(false)
    }

    /// Assigns a role to an account.
    fn add_role(&mut self, account_id: &AccountId, role: &R) {
        Self::root()
            .ns(role.try_to_vec().unwrap())
            .field(account_id.try_to_vec().unwrap())
            .write(&true);
    }

    /// Removes a role from an account.
    fn remove_role(&mut self, account_id: &AccountId, role: &R) {
        Self::root()
            .ns(role.try_to_vec().unwrap())
            .field::<_, bool>(account_id.try_to_vec().unwrap())
            .remove();
    }

    /// Requires transaction predecessor to have a given role.
    fn require_role(&self, role: &R) {
        let predecessor = env::predecessor_account_id();
        require!(Self::has_role(&predecessor, role), "Unauthorized");
    }

    /// Requires transaction predecessor to not have a given role.
    fn prohibit_role(&self, role: &R) {
        let predecessor = env::predecessor_account_id();
        require!(!Self::has_role(&predecessor, role), "Prohibited");
    }
}

#[cfg(test)]
mod tests {
    use near_contract_tools_macros::Rbac;
    use near_sdk::{
        borsh::{self, BorshSerialize},
        near_bindgen,
        test_utils::VMContextBuilder,
        testing_env, AccountId, BorshStorageKey,
    };

    use super::Rbac;

    mod near_contract_tools {
        pub use crate::*;
    }

    #[derive(BorshSerialize, BorshStorageKey)]
    enum Role {
        A,
        B,
    }

    #[derive(Rbac)]
    #[rbac(roles = "Role")]
    #[near_bindgen]
    struct Contract {}

    #[test]
    pub fn empty() {
        let a: AccountId = "account".parse().unwrap();

        assert!(!Contract::has_role(&a, &Role::A));
        assert!(!Contract::has_role(&a, &Role::B));
    }

    #[test]
    pub fn add_role() {
        let mut r = Contract {};
        let a: AccountId = "account".parse().unwrap();

        r.add_role(&a, &Role::A);

        assert!(Contract::has_role(&a, &Role::A));
        assert!(!Contract::has_role(&a, &Role::B));
    }

    #[test]
    pub fn remove_role() {
        let mut r = Contract {};
        let a: AccountId = "account".parse().unwrap();

        r.add_role(&a, &Role::B);
        r.add_role(&a, &Role::A);

        assert!(Contract::has_role(&a, &Role::A));
        assert!(Contract::has_role(&a, &Role::B));

        r.remove_role(&a, &Role::B);

        assert!(Contract::has_role(&a, &Role::A));
        assert!(!Contract::has_role(&a, &Role::B));
    }

    #[test]
    pub fn multiple_accounts() {
        let mut r = Contract {};
        let a: AccountId = "account_a".parse().unwrap();
        let b: AccountId = "account_b".parse().unwrap();

        r.add_role(&a, &Role::B);
        r.add_role(&a, &Role::A);
        r.add_role(&b, &Role::B);
        r.add_role(&b, &Role::A);

        assert!(Contract::has_role(&a, &Role::A));
        assert!(Contract::has_role(&a, &Role::B));
        assert!(Contract::has_role(&b, &Role::A));
        assert!(Contract::has_role(&b, &Role::B));

        r.remove_role(&a, &Role::B);
        r.remove_role(&b, &Role::A);

        assert!(Contract::has_role(&a, &Role::A));
        assert!(!Contract::has_role(&a, &Role::B));
        assert!(!Contract::has_role(&b, &Role::A));
        assert!(Contract::has_role(&b, &Role::B));
    }

    #[test]
    pub fn require_role_success() {
        let mut r = Contract {};
        let a: AccountId = "account".parse().unwrap();

        r.add_role(&a, &Role::A);

        testing_env!(VMContextBuilder::new().predecessor_account_id(a).build());

        r.require_role(&Role::A);
    }

    #[test]
    #[should_panic = "Unauthorized"]
    pub fn require_role_fail_wrong_role() {
        let mut r = Contract {};
        let a: AccountId = "account".parse().unwrap();

        r.add_role(&a, &Role::A);

        testing_env!(VMContextBuilder::new().predecessor_account_id(a).build());

        r.require_role(&Role::B);
    }

    #[test]
    #[should_panic = "Unauthorized"]
    pub fn require_role_fail_no_role() {
        let r = Contract {};
        let a: AccountId = "account".parse().unwrap();

        testing_env!(VMContextBuilder::new().predecessor_account_id(a).build());

        r.require_role(&Role::B);
    }

    #[test]
    #[should_panic = "Prohibited"]
    pub fn prohibit_role_fail() {
        let mut r = Contract {};
        let a: AccountId = "account".parse().unwrap();

        r.add_role(&a, &Role::A);

        testing_env!(VMContextBuilder::new().predecessor_account_id(a).build());

        r.prohibit_role(&Role::A);
    }

    #[test]
    pub fn prohibit_role_success_diff_role() {
        let mut r = Contract {};
        let a: AccountId = "account".parse().unwrap();

        r.add_role(&a, &Role::A);

        testing_env!(VMContextBuilder::new().predecessor_account_id(a).build());

        r.prohibit_role(&Role::B);
    }

    #[test]
    pub fn prohibit_role_success_no_role() {
        let r = Contract {};
        let a: AccountId = "account".parse().unwrap();

        testing_env!(VMContextBuilder::new().predecessor_account_id(a).build());

        r.prohibit_role(&Role::B);
    }
}
