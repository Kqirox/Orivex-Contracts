use soroban_sdk::{contracttype, Address, BytesN};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Course {
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
    /// Running count of all Progress entries currently in storage.
    /// Incremented on enroll, decremented on sweep removal. Used by
    /// `estimated_storage_footprint` without needing a full scan.
    ProgressCount,
    /// Append-only index of (learner, course_id) pairs where the learner
    /// has finished the course. Entries here are candidates for reclamation
    /// via `sweep_storage`. Stored as a `Vec<(Address, u32)>` under a
    /// single key so iteration during sweep is cheap.
    FinishedProgressIndex,
}
