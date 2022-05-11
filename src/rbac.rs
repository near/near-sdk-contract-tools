//! Role-based Access Control module

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LookupMap, LookupSet},
    env, require, AccountId, BorshStorageKey, IntoStorageKey,
};

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    RoleMap,
    RoleSet(Vec<u8>),
}

/// Role-based access control.
/// Parameterize with an enum of roles with
/// `#[derive(BorshSerialize, BorshStorageKey)]`
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Rbac<R: BorshSerialize + IntoStorageKey> {
    storage_key_prefix: Vec<u8>,
    roles: LookupMap<R, LookupSet<AccountId>>,
}

impl<R> Rbac<R>
where
    R: BorshSerialize + IntoStorageKey,
{
    /// Creates a new role-based access controller.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_contract_tools::rbac::Rbac;
    /// use near_sdk::{borsh::{self, BorshSerialize}, AccountId, BorshStorageKey};
    ///
    /// #[derive(BorshSerialize, BorshStorageKey)]
    /// enum Role {
    ///     A,
    ///     B,
    /// }
    ///
    /// let r = Rbac::new(b"r");
    /// let account: AccountId = "account".parse().unwrap();
    /// assert!(!r.has_role(&account, &Role::A));
    /// ```
    pub fn new<S>(storage_key_prefix: S) -> Self
    where
        S: IntoStorageKey,
    {
        let storage_key_prefix = storage_key_prefix.into_storage_key();
        let roles_prefix = vec![
            storage_key_prefix.clone(),
            StorageKey::RoleMap.into_storage_key(),
        ]
        .concat();

        Self {
            storage_key_prefix,
            roles: LookupMap::new(roles_prefix),
        }
    }

    /// Returns whether a given account has been given a certain role.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_contract_tools::rbac::Rbac;
    /// use near_sdk::{borsh::{self, BorshSerialize}, AccountId, BorshStorageKey};
    ///
    /// #[derive(BorshSerialize, BorshStorageKey)]
    /// enum Role {
    ///     A,
    ///     B,
    /// }
    ///
    /// let r = Rbac::new(b"r");
    /// let account: AccountId = "account".parse().unwrap();
    /// assert!(!r.has_role(&account, &Role::A));
    /// ```
    pub fn has_role(&self, account_id: &AccountId, role: &R) -> bool {
        if let Some(exists) = self.roles.get(role).map(|list| list.contains(account_id)) {
            exists
        } else {
            false
        }
    }

    /// Assigns a role to an account.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_contract_tools::rbac::Rbac;
    /// use near_sdk::{borsh::{self, BorshSerialize}, AccountId, BorshStorageKey};
    ///
    /// #[derive(BorshSerialize, BorshStorageKey)]
    /// enum Role {
    ///     A,
    ///     B,
    /// }
    ///
    /// let mut r = Rbac::new(b"r");
    /// let account: AccountId = "account".parse().unwrap();
    /// r.add_role(&account, &Role::A);
    /// assert!(r.has_role(&account, &Role::A));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if role is not serializable.
    pub fn add_role(&mut self, account_id: &AccountId, role: &R) {
        if let Some(mut list) = self.roles.get(role) {
            list.insert(account_id);
        } else {
            let prefix = vec![
                self.storage_key_prefix.clone(),
                StorageKey::RoleSet(role.try_to_vec().unwrap()).into_storage_key(),
            ]
            .concat();
            let mut list = LookupSet::<AccountId>::new(prefix);
            list.insert(account_id);
            self.roles.insert(role, &list);
        }
    }

    /// Removes a role from an account.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_contract_tools::rbac::Rbac;
    /// use near_sdk::{borsh::{self, BorshSerialize}, AccountId, BorshStorageKey};
    ///
    /// #[derive(BorshSerialize, BorshStorageKey)]
    /// enum Role {
    ///     A,
    ///     B,
    /// }
    ///
    /// let mut r = Rbac::new(b"r");
    /// let account: AccountId = "account".parse().unwrap();
    /// r.add_role(&account, &Role::A);
    /// assert!(r.has_role(&account, &Role::A));
    /// r.remove_role(&account, &Role::A);
    /// assert!(!r.has_role(&account, &Role::A));
    /// ```
    pub fn remove_role(&mut self, account_id: &AccountId, role: &R) {
        if let Some(mut list) = self.roles.get(role) {
            list.remove(account_id);
        }
    }

    /// Requires transaction predecessor to have a given role.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_contract_tools::rbac::Rbac;
    /// use near_sdk::{borsh::{self, BorshSerialize}, AccountId, BorshStorageKey, test_utils::VMContextBuilder, testing_env};
    ///
    /// #[derive(BorshSerialize, BorshStorageKey)]
    /// enum Role {
    ///     A,
    ///     B,
    /// }
    ///
    /// let mut r = Rbac::new(b"r");
    /// let account: AccountId = "account".parse().unwrap();
    /// r.add_role(&account, &Role::A);
    /// testing_env!(VMContextBuilder::new().predecessor_account_id(account).build());
    /// r.require_role(&Role::A);
    /// ```
    pub fn require_role(&self, role: &R) {
        let predecessor = env::predecessor_account_id();
        require!(self.has_role(&predecessor, role), "Unauthorized");
    }
}

#[cfg(test)]
mod tests {
    use near_sdk::{
        borsh::{self, BorshSerialize},
        test_utils::VMContextBuilder,
        testing_env, AccountId, BorshStorageKey,
    };

    use super::Rbac;

    #[derive(BorshSerialize, BorshStorageKey)]
    enum Role {
        A,
        B,
    }

    #[test]
    pub fn empty() {
        let r = Rbac::<Role>::new(b"r");
        let a: AccountId = "account".parse().unwrap();

        assert!(!r.has_role(&a, &Role::A));
        assert!(!r.has_role(&a, &Role::B));
    }

    #[test]
    pub fn add_role() {
        let mut r = Rbac::<Role>::new(b"r");
        let a: AccountId = "account".parse().unwrap();

        r.add_role(&a, &Role::A);

        assert!(r.has_role(&a, &Role::A));
        assert!(!r.has_role(&a, &Role::B));
    }

    #[test]
    pub fn remove_role() {
        let mut r = Rbac::<Role>::new(b"r");
        let a: AccountId = "account".parse().unwrap();

        r.add_role(&a, &Role::B);
        r.add_role(&a, &Role::A);

        assert!(r.has_role(&a, &Role::A));
        assert!(r.has_role(&a, &Role::B));

        r.remove_role(&a, &Role::B);

        assert!(r.has_role(&a, &Role::A));
        assert!(!r.has_role(&a, &Role::B));
    }

    #[test]
    pub fn multiple_accounts() {
        let mut r = Rbac::<Role>::new(b"r");
        let a: AccountId = "account_a".parse().unwrap();
        let b: AccountId = "account_b".parse().unwrap();

        r.add_role(&a, &Role::B);
        r.add_role(&a, &Role::A);
        r.add_role(&b, &Role::B);
        r.add_role(&b, &Role::A);

        assert!(r.has_role(&a, &Role::A));
        assert!(r.has_role(&a, &Role::B));
        assert!(r.has_role(&b, &Role::A));
        assert!(r.has_role(&b, &Role::B));

        r.remove_role(&a, &Role::B);
        r.remove_role(&b, &Role::A);

        assert!(r.has_role(&a, &Role::A));
        assert!(!r.has_role(&a, &Role::B));
        assert!(!r.has_role(&b, &Role::A));
        assert!(r.has_role(&b, &Role::B));
    }

    #[test]
    pub fn require_role_success() {
        let mut r = Rbac::<Role>::new(b"r");
        let a: AccountId = "account".parse().unwrap();

        r.add_role(&a, &Role::A);

        testing_env!(VMContextBuilder::new().predecessor_account_id(a).build());

        r.require_role(&Role::A);
    }

    #[test]
    #[should_panic = "Unauthorized"]
    pub fn require_role_fail_wrong_role() {
        let mut r = Rbac::<Role>::new(b"r");
        let a: AccountId = "account".parse().unwrap();

        r.add_role(&a, &Role::A);

        testing_env!(VMContextBuilder::new().predecessor_account_id(a).build());

        r.require_role(&Role::B);
    }

    #[test]
    #[should_panic = "Unauthorized"]
    pub fn require_role_fail_no_role() {
        let r = Rbac::<Role>::new(b"r");
        let a: AccountId = "account".parse().unwrap();

        testing_env!(VMContextBuilder::new().predecessor_account_id(a).build());

        r.require_role(&Role::B);
    }
}
