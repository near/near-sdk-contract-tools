//! Contract ownership pattern

use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::LazyOption,
    env, require, AccountId, IntoStorageKey,
};

use crate::utils::prefix_key;

/// State for contract ownership management
///
/// # Examples
///
/// ```
/// use near_contract_tools::ownership::Ownership;
///
/// struct Contract {
///     // ...
///     pub ownership: Ownership,
/// }
/// ```
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Ownership {
    /// The current owner of the contract.
    /// Will be `None` if the current owner has renounced ownership.
    pub owner: Option<AccountId>,
    /// Proposed owner, if current owner has proposed a new owner.
    /// For 2-step power transition:
    /// 1. Current owner must propose a new owner
    /// 2. New owner must accept ownership
    pub proposed_owner: LazyOption<AccountId>,
}

impl Ownership {
    /// Creates a new Ownership using the specified storage key prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_contract_tools::ownership::Ownership;
    ///
    /// let ownership = Ownership::new(
    ///     b"o",
    ///     near_sdk::env::predecessor_account_id(),
    /// );
    /// ```
    pub fn new<S>(storage_key_prefix: S, owner_id: AccountId) -> Self
    where
        S: IntoStorageKey,
    {
        let k = storage_key_prefix.into_storage_key();

        Self {
            owner: Some(owner_id),
            proposed_owner: LazyOption::new(prefix_key(&k, b"p"), None),
        }
    }

    /// Requires the predecessor to be the owner
    ///
    /// # Examples
    ///
    /// ```
    /// use near_contract_tools::ownership::Ownership;
    ///
    /// let ownership = Ownership::new(
    ///     b"o",
    ///     near_sdk::env::predecessor_account_id(),
    /// );
    /// ownership.require_owner();
    /// ```
    pub fn require_owner(&self) {
        require!(
            &env::predecessor_account_id()
                == self
                    .owner
                    .as_ref()
                    .unwrap_or_else(|| env::panic_str("No owner")),
            "Owner only"
        );
    }

    /// Removes the contract's owner. Can only be called by the current owner.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_contract_tools::ownership::Ownership;
    ///
    /// let owner_id = near_sdk::env::predecessor_account_id();
    /// let mut ownership = Ownership::new(
    ///     b"o",
    ///     owner_id.clone(),
    /// );
    /// assert_eq!(ownership.owner, Some(owner_id));
    /// ownership.renounce_owner();
    /// assert_eq!(ownership.owner, None);
    /// ```
    pub fn renounce_owner(&mut self) {
        self.require_owner();
        self.owner = None;
        self.proposed_owner.remove();
    }

    /// Prepares the contract to change owners, setting the proposed owner to
    /// the provided account ID. Can only be called by the current owner.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_contract_tools::ownership::Ownership;
    ///
    /// let mut ownership = Ownership::new(
    ///     b"o",
    ///     near_sdk::env::predecessor_account_id(),
    /// );
    /// let proposed_owner: near_sdk::AccountId = "account".parse().unwrap();
    /// assert_eq!(ownership.proposed_owner.get(), None);
    /// ownership.propose_owner(Some(proposed_owner.clone()));
    /// assert_eq!(ownership.proposed_owner.get(), Some(proposed_owner));
    /// ```
    pub fn propose_owner(&mut self, account_id: Option<AccountId>) {
        self.require_owner();
        if let Some(a) = account_id {
            self.proposed_owner.set(&a);
        } else {
            self.proposed_owner.remove();
        }
    }

    /// Sets new owner equal to proposed owner. Can only be called by proposed
    /// owner.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_contract_tools::ownership::Ownership;
    ///
    /// let owner_id = "account".parse().unwrap();
    /// let mut ownership = Ownership::new(
    ///     b"o",
    ///     owner_id,
    /// );
    /// let proposed_owner = near_sdk::env::predecessor_account_id();
    /// ownership.proposed_owner.set(&proposed_owner);
    /// ownership.accept_owner();
    /// assert_eq!(ownership.owner, Some(proposed_owner));
    /// ```
    pub fn accept_owner(&mut self) {
        let proposed_owner = self
            .proposed_owner
            .take()
            .unwrap_or_else(|| env::panic_str("No proposed owner"));
        require!(
            env::predecessor_account_id() == proposed_owner,
            "Proposed owner only"
        );
        self.owner = Some(proposed_owner);
    }
}

/// A contract that conforms to the ownership pattern as described in this
/// crate will implement this trait.
pub trait Ownable {
    /// Returns the account ID of the current owner
    fn own_get_owner(&self) -> Option<AccountId>;
    /// Returns the account ID that the current owner has proposed take over ownership
    fn own_get_proposed_owner(&self) -> Option<AccountId>;
    /// Current owner may call this function to renounce ownership, setting
    /// current owner to `None`.
    ///
    /// **WARNING**: Once this function has been called, this implementation
    /// does not provide a way for the contract to have an owner again!
    fn own_renounce_owner(&mut self);
    /// Propose a new owner. Can only be called by the current owner
    fn own_propose_owner(&mut self, account_id: Option<AccountId>);
    /// The proposed owner may call this function to accept ownership from the
    /// previous owner
    fn own_accept_owner(&mut self);
}

/// Implements the ownership pattern on a contract struct
///
/// # Examples
///
/// ```
/// use near_sdk::{
///     near_bindgen,
///     AccountId,
///     assert_one_yocto,
/// };
/// use near_contract_tools::{
///     impl_ownership,
///     ownership::Ownership,
/// };
///
/// #[near_bindgen]
/// struct Contract {
///     pub ownership: Ownership,
/// }
///
/// impl_ownership!(Contract, ownership);
/// ```
#[macro_export]
macro_rules! impl_ownership {
    ($contract: ident, $ownership: ident) => {
        use $crate::ownership::Ownable;

        #[near_bindgen]
        impl Ownable for $contract {
            fn own_get_owner(&self) -> Option<AccountId> {
                self.$ownership.owner.clone()
            }

            fn own_get_proposed_owner(&self) -> Option<AccountId> {
                self.$ownership.proposed_owner.get()
            }

            #[payable]
            fn own_renounce_owner(&mut self) {
                assert_one_yocto();
                self.$ownership.renounce_owner()
            }

            #[payable]
            fn own_propose_owner(&mut self, account_id: Option<AccountId>) {
                assert_one_yocto();
                self.$ownership.propose_owner(account_id);
            }

            #[payable]
            fn own_accept_owner(&mut self) {
                assert_one_yocto();
                self.$ownership.accept_owner();
            }
        }
    };
}
