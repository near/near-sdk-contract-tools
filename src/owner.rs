//! Owner pattern

use near_contract_tools_macros::Event;
use near_sdk::{env, require, AccountId};
use serde::Serialize;

use crate::{event::Event, near_contract_tools, slot::Slot};

/// Events emitted by function calls on an ownable contract
#[derive(Event, Serialize)]
#[event(standard = "x-own", version = "1.0.0", rename_all = "snake_case")]
#[serde(untagged)]
pub enum OwnerEvent {
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

/// A contract with an owner
pub trait Owner {
    /// Storage root
    fn root(&self) -> Slot<()>;

    /// Storage slot for initialization state
    fn slot_is_initialized(&self) -> Slot<bool> {
        self.root().field(b"i")
    }

    /// Storage slot for owner account ID
    fn slot_owner(&self) -> Slot<AccountId> {
        self.root().field(b"o")
    }

    /// Storage slot for proposed owner account ID
    fn slot_proposed_owner(&self) -> Slot<AccountId> {
        self.root().field(b"p")
    }

    /// Updates the current owner and emits relevant event
    fn update_owner(&mut self, new: Option<AccountId>) {
        let mut owner = self.slot_owner();
        let old = owner.read();
        if old != new {
            OwnerEvent::Transfer {
                old,
                new: new.clone(),
            }
            .emit();
            owner.set(new.as_ref());
        }
    }

    /// Updates proposed owner and emits relevant event
    fn update_proposed(&mut self, new: Option<AccountId>) {
        let mut proposed_owner = self.slot_proposed_owner();
        let old = proposed_owner.read();
        if old != new {
            OwnerEvent::Propose {
                old,
                new: new.clone(),
            }
            .emit();
            proposed_owner.set(new.as_ref());
        }
    }

    /// Initializes the contract owner. Can only be called once.
    ///
    /// Emits an `OwnerEvent::Transfer` event.
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::{AccountId, near_bindgen};
    /// use near_contract_tools::{Owner, owner::Owner};
    ///
    /// #[derive(Owner)]
    /// #[near_bindgen]
    /// struct Contract {}
    ///
    /// #[near_bindgen]
    /// impl Contract {
    ///     pub fn new(owner_id: AccountId) -> Self {
    ///         let contract = Self {};
    ///
    ///         Owner::init(&contract, &owner_id);
    ///
    ///         contract
    ///     }
    /// }
    /// ```
    fn init(&self, owner_id: &AccountId) {
        require!(
            !self.slot_is_initialized().exists(),
            "Owner already initialized",
        );

        self.slot_is_initialized().write(&true);
        self.slot_owner().write(owner_id);

        OwnerEvent::Transfer {
            old: None,
            new: Some(owner_id.clone()),
        }
        .emit();
    }

    /// Requires the predecessor to be the owner
    ///
    /// # Examples
    ///
    /// ```
    /// use near_sdk::{AccountId, near_bindgen};
    /// use near_contract_tools::{Owner, owner::Owner};
    ///
    /// #[derive(Owner)]
    /// #[near_bindgen]
    /// struct Contract {}
    ///
    /// #[near_bindgen]
    /// impl Contract {
    ///     pub fn owner_only(&self) {
    ///         self.require_owner();
    ///
    ///         // ...
    ///     }
    /// }
    /// ```
    fn require_owner(&self) {
        require!(
            &env::predecessor_account_id()
                == self
                    .slot_owner()
                    .read()
                    .as_ref()
                    .unwrap_or_else(|| env::panic_str("No owner")),
            "Owner only",
        );
    }

    /// Removes the contract's owner. Can only be called by the current owner.
    ///
    /// Emits an `OwnerEvent::Transfer` event, and an `OwnerEvent::Propose`
    /// event if there is a currently proposed owner.
    fn renounce_owner(&mut self) {
        self.require_owner();

        self.update_proposed(None);
        self.update_owner(None);
    }

    /// Prepares the contract to change owners, setting the proposed owner to
    /// the provided account ID. Can only be called by the current owner.
    ///
    /// Emits an `OwnerEvent::Propose` event.
    ///
    /// The currently proposed owner may be reset by calling this function with
    /// the argument `None`.
    fn propose_owner(&mut self, account_id: Option<AccountId>) {
        self.require_owner();

        self.update_proposed(account_id);
    }

    /// Sets new owner equal to proposed owner. Can only be called by proposed
    /// owner.
    ///
    /// Emits events corresponding to the transfer of ownership and reset of the
    /// proposed owner.
    fn accept_owner(&mut self) {
        let proposed_owner = self
            .slot_proposed_owner()
            .take()
            .unwrap_or_else(|| env::panic_str("No proposed owner"));

        require!(
            env::predecessor_account_id() == proposed_owner,
            "Proposed owner only",
        );

        OwnerEvent::Propose {
            old: Some(proposed_owner.clone()),
            new: None,
        }
        .emit();

        self.update_owner(Some(proposed_owner));
    }
}

/// Externally-accessible functions for `Owner`
pub trait OwnerExternal {
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

#[cfg(test)]
mod tests {
    use near_sdk::{near_bindgen, test_utils::VMContextBuilder, testing_env, AccountId};

    use crate::{
        owner::{Owner, OwnerExternal},
        Owner,
    };

    mod near_contract_tools {
        pub use crate::*;
    }

    #[derive(Owner)]
    #[near_bindgen]
    struct Contract {}

    #[near_bindgen]
    impl Contract {
        #[init]
        pub fn new(owner_id: AccountId) -> Self {
            let contract = Self {};

            Owner::init(&contract, &owner_id);

            contract
        }

        pub fn owner_only(&self) {
            self.require_owner();
        }
    }

    #[test]
    fn require_owner() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let contract = Contract::new(owner_id.clone());

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id)
            .build());

        contract.owner_only();
    }

    #[test]
    #[should_panic(expected = "Owner only")]
    fn require_owner_fail() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let contract = Contract::new(owner_id.clone());

        let alice: AccountId = "alice".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(alice)
            .build());

        contract.owner_only();
    }

    #[test]
    fn renounce_owner() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let mut contract = Contract::new(owner_id.clone());
        assert_eq!(contract.own_get_owner(), Some(owner_id.clone()));
        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id)
            .attached_deposit(1)
            .build());
        contract.own_renounce_owner();
        assert_eq!(contract.own_get_owner(), None);
    }

    #[test]
    fn propose_owner() {
        let owner_id: AccountId = "owner".parse().unwrap();
        let mut contract = Contract::new(owner_id.clone());

        let proposed_owner: AccountId = "proposed".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id.clone())
            .attached_deposit(1)
            .build());

        assert_eq!(contract.own_get_proposed_owner(), None);

        contract.own_propose_owner(Some(proposed_owner.clone()));

        assert_eq!(contract.own_get_proposed_owner(), Some(proposed_owner));
    }

    #[test]
    #[should_panic(expected = "Owner only")]
    fn propose_owner_unauthorized() {
        let owner_id: AccountId = "owner".parse().unwrap();
        let mut contract = Contract::new(owner_id.clone());

        let proposed_owner: AccountId = "proposed".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(proposed_owner.clone())
            .attached_deposit(1)
            .build());

        contract.own_propose_owner(Some(proposed_owner.clone()));
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn propose_owner_no_deposit() {
        let owner_id: AccountId = "owner".parse().unwrap();
        let mut contract = Contract::new(owner_id.clone());

        let proposed_owner: AccountId = "proposed".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id.clone())
            .build());

        contract.own_propose_owner(Some(proposed_owner.clone()));
    }

    #[test]
    fn accept_owner() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let mut contract = Contract::new(owner_id.clone());

        let proposed_owner: AccountId = "proposed".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id.clone())
            .attached_deposit(1)
            .build());

        contract.own_propose_owner(Some(proposed_owner.clone()));

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(proposed_owner.clone())
            .attached_deposit(1)
            .build());

        contract.own_accept_owner();

        assert_eq!(contract.own_get_owner(), Some(proposed_owner));
        assert_eq!(contract.own_get_proposed_owner(), None);
    }

    #[test]
    #[should_panic(expected = "Proposed owner only")]
    fn accept_owner_unauthorized() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let mut contract = Contract::new(owner_id.clone());

        let proposed_owner: AccountId = "proposed".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id.clone())
            .attached_deposit(1)
            .build());

        contract.own_propose_owner(Some(proposed_owner.clone()));

        let third_party: AccountId = "third".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(third_party.clone())
            .attached_deposit(1)
            .build());

        contract.own_accept_owner();
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn accept_owner_no_deposit() {
        let owner_id: AccountId = "owner".parse().unwrap();

        let mut contract = Contract::new(owner_id.clone());

        let proposed_owner: AccountId = "proposed".parse().unwrap();

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(owner_id.clone())
            .attached_deposit(1)
            .build());

        contract.own_propose_owner(Some(proposed_owner.clone()));

        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(proposed_owner.clone())
            .build());

        contract.own_accept_owner();
    }
}
