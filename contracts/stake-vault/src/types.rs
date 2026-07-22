use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakeInfo {
    pub amount: i128,
    pub lock_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Token,
    UserStake(Address),
    /// Running count of active UserStake entries in persistent storage.
    /// Incremented on the first `stake` call for a user; decremented on
    /// `unstake` (which removes the entry). Used by
    /// `estimated_storage_footprint`.
    StakerCount,
}
