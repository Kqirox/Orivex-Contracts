use soroban_sdk::{contracttype, Address, BytesN};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Course {
    pub instructor: Address,
    pub total_modules: u32,
    pub metadata_hash: BytesN<32>,
    pub active: bool,
}

/// Schema used before VERSION 1 was introduced (v0 layout).
/// Kept here so the `migrate()` function can decode v0 Course records
/// and re-encode them as the current `Course` struct.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CourseV0 {
    pub instructor: Address,
    pub total_modules: u32,
    pub metadata_hash: BytesN<32>,
    pub active: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Course(u32),
    Progress(Address, u32),
    CourseCount,
    Admin,
    BadgeNftAddress,
    RewardPoolAddress,
    /// Tracks a pending reward for a learner who completed a course but
    /// whose reward payout failed. The learner can call
    /// `claim_completion_reward` to retry.
    PendingReward(Address, u32),
    /// Monotonically increasing schema version stored in instance storage.
    /// 0  = pre-versioning (no Version key present).
    /// 1  = current schema (this build).
    Version,
}
