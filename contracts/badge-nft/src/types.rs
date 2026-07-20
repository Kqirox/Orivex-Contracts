use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Badge {
    pub course_id: u32,
    pub minted_at: u64,
}

/// Schema used before VERSION 1 was introduced (v0 layout).
/// Wire-compatible with `Badge`; kept for documentation clarity.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BadgeV0 {
    pub course_id: u32,
    pub minted_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    UserBadges(Address),
    /// Monotonically increasing schema version stored in instance storage.
    /// 0  = pre-versioning (no Version key present).
    /// 1  = current schema (this build).
    Version,
}
