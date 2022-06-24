//! Contract ownership pattern

use near_contract_tools_macros::Event;
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::LazyOption,
    env, require, AccountId, IntoStorageKey,
};
use serde::Serialize;

use crate::{event::Event, near_contract_tools, utils::prefix_key};

/// Events emitted by function calls on an ownable contract
#[derive(Event, Serialize)]
#[event(standard = "x-own", version = "1.0.0", rename_all = "snake_case")]
#[serde(untagged)]
pub enum OwnershipEvent {
    /// Emitted when the current owner of the contract changes
    Transfer {
        /// Former owner of the contract. Will be `None` if the contract is being initialized.
        old: Option<AccountId>,
        /// The new owner of the contract. Will be `None` if ownership is renounced.
        new: Option<AccountId>,
    },
    /// Emitted when the proposed owner of the contract changes
    Propose {
        /// Old proposed owner.
        old: Option<AccountId>,
        /// New proposed owner.
        new: Option<AccountId>,
    },
}

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
    /// Updates the current owner and emits relevant event
    fn update_owner(&mut self, new: Option<AccountId>) {
        let old = self.owner.clone();
        if old != new {
            OwnershipEvent::Transfer {
                old,
                new: new.clone(),
            }
            .emit();
            self.owner = new;
        }
    }

    /// Updates proposed owner and emits relevant event
    fn update_proposed(&mut self, new: Option<AccountId>) {
        let old = self.proposed_owner.get();
        if old != new {
            OwnershipEvent::Propose {
                old,
                new: new.clone(),
            }
            .emit();
            match new {
                Some(account_id) => self.proposed_owner.set(&account_id),
                _ => self.proposed_owner.remove(),
            };
        }
    }

    /// Creates a new Ownership using the specified storage key prefix.
    ///
    /// Emits an `OwnershipEvent::Transfer` event.
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

        OwnershipEvent::Transfer {
            old: None,
            new: Some(owner_id.clone()),
        }
        .emit();

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
    /// Emits an `OwnershipEvent::Transfer` event, and an `OwnershipEvent::Propose` event
    /// if there is a currently proposed owner.
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

        self.update_proposed(None);
        self.update_owner(None);
    }

    /// Prepares the contract to change owners, setting the proposed owner to
    /// the provided account ID. Can only be called by the current owner.
    ///
    /// Emits an `OwnershipEvent::Propose` event.
    ///
    /// The currently proposed owner may be reset by calling this function with the argument `None`.
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

        self.update_proposed(account_id);
    }

    /// Sets new owner equal to proposed owner. Can only be called by proposed
    /// owner.
    ///
    /// Emits events corresponding to the transfer of ownership and reset of the
    /// proposed owner.
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

        OwnershipEvent::Propose {
            old: Some(proposed_owner.clone()),
            new: None,
        }
        .emit();

        self.update_owner(Some(proposed_owner));
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

/// Internal management for derive macro
pub trait OwnershipController {
    /// Initialization method. May only be called once.
    fn init_owner(&self, owner_id: AccountId) -> Ownership;

    /// Get the ownership struct from storage
    fn get_ownership(&self) -> Ownership;

    /// Requires that the predecessor is the owner; rejects otherwise.
    fn require_owner(&self) -> Ownership {
        let ownership = self.get_ownership();
        ownership.require_owner();
        ownership
    }
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
