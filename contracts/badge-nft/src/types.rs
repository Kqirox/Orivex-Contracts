use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Badge {
    pub course_id: u32,
    pub minted_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    UserBadges(Address),
    /// Running count of unique learner addresses that have at least one badge.
    /// Incremented the first time a learner's badge vec is created; never
    /// decremented (revocation reduces vec length, but the key stays).
    /// Used by `estimated_storage_footprint`.
    BadgeHolderCount,
}
