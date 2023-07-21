use crate::{event, standard::nep297::Event};
use crate::{slot::Slot, DefaultStorageKey};
use near_sdk::{
    borsh::BorshSerialize,
    borsh::{self, BorshDeserialize},
    env::panic_str,
    require,
    serde::Serialize,
    BorshStorageKey,
};

const ESCROW_ALREADY_LOCKED_MESSAGE: &str = "Already locked";
const ESCROW_NOT_LOCKED_MESSAGE: &str = "Lock required";
const ESCROW_LOCK_HANDLER_FAILED_MESSAGE: &str = "Lock handler failed, not unlocking";

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey<'a, T> {
    Locked(&'a T),
}

/// Emit the state of an escrow lock and whether it was locked or unlocked
#[event(
    standard = "x-escrow",
    version = "1.0.0",
    crate = "crate",
    macros = "near_sdk_contract_tools_macros"
)]
pub struct Lock<Id: Serialize, State: Serialize> {
    /// The identifier for a lock
    pub id: Id,
    /// If the lock was locked or unlocked, and any state along with it
    pub locked: Option<State>,
}

/// Inner storage modifiers and functionality required for escrow to succeed
pub trait EscrowInternal {
    /// Identifier over which the escrow exists
    type Id: BorshSerialize;
    /// State stored inside the lock
    type State: BorshSerialize + BorshDeserialize;

    /// Retrieve the state root
    fn root() -> Slot<()> {
        Slot::root(DefaultStorageKey::Escrow)
    }

    /// Inner function to retrieve the slot keyed by it's `Self::Id`
    fn locked_slot(&self, id: &Self::Id) -> Slot<Self::State> {
        Self::root().field(StorageKey::Locked(id))
    }

    /// Read the state from the slot
    fn get_locked(&self, id: &Self::Id) -> Option<Self::State> {
        self.locked_slot(id).read()
    }

    /// Set the state at `id` to `locked`
    fn set_locked(&mut self, id: &Self::Id, locked: &Self::State) {
        self.locked_slot(id).write(locked);
    }

    /// Clear the state at `id`
    fn set_unlocked(&mut self, id: &Self::Id) {
        self.locked_slot(id).remove();
    }
}

/// Some escrowable capabilities, with a simple locking/unlocking mechanism
/// If you add additional `Approve` capabilities here, you can make use of a step-wise locking system.
pub trait Escrow {
    /// Identifier over which the escrow exists
    type Id: BorshSerialize;
    /// State stored inside the lock
    type State: BorshSerialize + BorshDeserialize;

    /// Lock some `Self::State` by it's `Self::Id` within the store
    fn lock(&mut self, id: &Self::Id, state: &Self::State);

    /// Unlock and release some `Self::State` by it's `Self::Id`
    fn unlock(&mut self, id: &Self::Id, lock_handler: impl FnOnce(&Self::State) -> bool);

    /// Check if the item is locked
    fn is_locked(&self, id: &Self::Id) -> bool;
}

impl<T> Escrow for T
where
    T: EscrowInternal,
    <T as EscrowInternal>::Id: Serialize,
    <T as EscrowInternal>::State: Serialize,
{
    type Id = <Self as EscrowInternal>::Id;
    type State = <Self as EscrowInternal>::State;

    fn lock(&mut self, id: &Self::Id, state: &Self::State) {
        require!(self.get_locked(id).is_none(), ESCROW_ALREADY_LOCKED_MESSAGE);

        self.set_locked(id, state);
        Lock {
            id: id.to_owned(),
            locked: Some(state),
        }
        .emit();
    }

    fn unlock(&mut self, id: &Self::Id, lock_handler: impl FnOnce(&Self::State) -> bool) {
        let lock = self
            .get_locked(id)
            .unwrap_or_else(|| panic_str(ESCROW_NOT_LOCKED_MESSAGE));

        if lock_handler(&lock) {
            self.set_unlocked(id);
            Lock::<_, Self::State> { id, locked: None }.emit();
        } else {
            panic_str(ESCROW_LOCK_HANDLER_FAILED_MESSAGE)
        }
    }

    fn is_locked(&self, id: &Self::Id) -> bool {
        self.get_locked(id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::Escrow;
    use crate::escrow::EscrowInternal;
    use near_sdk::{
        near_bindgen, test_utils::VMContextBuilder, testing_env, AccountId, Balance, VMContext,
        ONE_YOCTO,
    };
    use near_sdk_contract_tools_macros::Escrow;

    const ID: u64 = 1;
    const IS_NOT_READY: bool = false;

    #[derive(Escrow)]
    #[escrow(id = "u64", state = "bool", crate = "crate")]
    #[near_bindgen]
    struct Contract {
        is_ready: bool,
    }

    #[near_bindgen]
    impl Contract {
        #[init]
        pub fn new() -> Self {
            Self { is_ready: false }
        }
    }

    fn get_context(attached_deposit: Balance, signer: Option<AccountId>) -> VMContext {
        VMContextBuilder::new()
            .signer_account_id(signer.clone().unwrap_or("alice".parse().unwrap()))
            .predecessor_account_id(signer.unwrap_or("alice".parse().unwrap()))
            .attached_deposit(attached_deposit)
            .is_view(false)
            .build()
    }

    #[test]
    fn test_can_lock() {
        testing_env!(get_context(ONE_YOCTO, None));
        let mut contract = Contract::new();

        contract.lock(&ID, &IS_NOT_READY);
        assert!(contract.get_locked(&ID).is_some());
    }

    #[test]
    #[should_panic(expected = "Already locked")]
    fn test_cannot_lock_twice() {
        testing_env!(get_context(ONE_YOCTO, None));
        let mut contract = Contract::new();

        contract.lock(&ID, &IS_NOT_READY);
        contract.lock(&ID, &IS_NOT_READY);
    }

    #[test]
    fn test_can_unlock() {
        testing_env!(get_context(ONE_YOCTO, None));
        let mut contract = Contract::new();

        let is_ready = true;
        contract.lock(&ID, &is_ready);
        contract.unlock(&ID, |readiness| readiness == &is_ready);

        assert!(contract.get_locked(&ID).is_none());
    }

    #[test]
    #[should_panic(expected = "Lock handler failed, not unlocking")]
    fn test_cannot_unlock_until_ready() {
        testing_env!(get_context(ONE_YOCTO, None));
        let mut contract = Contract::new();

        let is_ready = true;
        contract.lock(&ID, &IS_NOT_READY);
        contract.unlock(&ID, |readiness| readiness == &is_ready);

        assert!(contract.get_locked(&ID).is_none());
    }
}
