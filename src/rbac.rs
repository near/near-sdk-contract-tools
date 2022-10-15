//! Role-Based Access Control pattern implements methods to manage roles for
//! accounts and control their access.
//!
//! RBAC expects the user to provide a type for [`Role`][Rbac::Role]. Typically
//! this is an enum and it's variants are the distinct roles. An account can be
//! associated with multiple roles. [`Rbac`] implements methods to add, remove
//! and check an account for a role. It also provides "guard" methods to require
//! or prohibit a particular role. Typically these are used to guard access to
//! external functions exposed by the contract.
//!
//! This [derive_macro](near_contract_tools_macros::Rbac) derives
//! a default implementation for Rbac. For a complete example check out
//! [counter_multisig.rs](https://github.com/NEARFoundation/near-contract-tools/blob/develop/workspaces-tests/src/bin/counter_multisig.rs)
//! in workspace-tests directory.
//!
//! # Safety
//! The default implementation assumes or enforces the following invariants.
//! Violating assumed invariants may corrupt contract state and show unexpected
//! behavior [UB]. "guard" methods enforce invariants and throw an error [ERR]
//! when accessed by unauthorized accounts.
//!
//! * [UB] The rbac root storage slot is not used or modified. The default key is `~r`.
//! * [ERR] [`require_role`][Rbac::require_role] only allows accounts with the required
//!   role
//! * [ERR] [`prohibit_role`][Rbac::prohibit_role] does not allow accounts with
//!   the prohibited role
use near_sdk::{borsh::BorshSerialize, env, require, AccountId, IntoStorageKey};

use crate::slot::Slot;

const REQUIRE_ROLE_FAIL_MESSAGE: &str = "Unauthorized role";
const PROHIBIT_ROLE_FAIL_MESSAGE: &str = "Prohibited role";

/// Role-based access control
pub trait Rbac {
    /// Roles type (probably an enum)
    type Role: BorshSerialize + IntoStorageKey;

    /// Storage slot namespace for items
    fn root() -> Slot<()>;

    /// Returns whether a given account has been given a certain role.
    fn has_role(account_id: &AccountId, role: &Self::Role) -> bool {
        Self::root()
            .ns(role.try_to_vec().unwrap())
            .field(account_id.try_to_vec().unwrap())
            .read()
            .unwrap_or(false)
    }

    /// Assigns a role to an account.
    fn add_role(&mut self, account_id: &AccountId, role: &Self::Role) {
        Self::root()
            .ns(role.try_to_vec().unwrap())
            .field(account_id.try_to_vec().unwrap())
            .write(&true);
    }

    /// Removes a role from an account.
    fn remove_role(&mut self, account_id: &AccountId, role: &Self::Role) {
        Self::root()
            .ns(role.try_to_vec().unwrap())
            .field::<_, bool>(account_id.try_to_vec().unwrap())
            .remove();
    }

    /// Requires transaction predecessor to have a given role.
    fn require_role(&self, role: &Self::Role) {
        let predecessor = env::predecessor_account_id();
        require!(
            Self::has_role(&predecessor, role),
            REQUIRE_ROLE_FAIL_MESSAGE
        );
    }

    /// Requires transaction predecessor to not have a given role.
    fn prohibit_role(&self, role: &Self::Role) {
        let predecessor = env::predecessor_account_id();
        require!(
            !Self::has_role(&predecessor, role),
            PROHIBIT_ROLE_FAIL_MESSAGE
        );
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
    #[rbac(roles = "Role", crate = "crate")]
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
    #[should_panic = "Unauthorized role"]
    pub fn require_role_fail_wrong_role() {
        let mut r = Contract {};
        let a: AccountId = "account".parse().unwrap();

        r.add_role(&a, &Role::A);

        testing_env!(VMContextBuilder::new().predecessor_account_id(a).build());

        r.require_role(&Role::B);
    }

    #[test]
    #[should_panic = "Unauthorized role"]
    pub fn require_role_fail_no_role() {
        let r = Contract {};
        let a: AccountId = "account".parse().unwrap();

        testing_env!(VMContextBuilder::new().predecessor_account_id(a).build());

        r.require_role(&Role::B);
    }

    #[test]
    #[should_panic = "Prohibited role"]
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
