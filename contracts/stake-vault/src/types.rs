use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakeInfo {
    pub amount: i128,
    pub lock_timestamp: u64,
}

/// Schema used before VERSION 1 was introduced (v0 layout).
/// Wire-compatible with `StakeInfo`; kept for documentation clarity.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakeInfoV0 {
    pub amount: i128,
    pub lock_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Token,
    UserStake(Address),
    /// Monotonically increasing schema version stored in instance storage.
    /// 0  = pre-versioning (no Version key present).
    /// 1  = current schema (this build).
    Version,
}
