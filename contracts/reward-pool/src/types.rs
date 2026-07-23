use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Token,
    Spender(Address),
    IsPaused,
    /// Running count of approved spender entries in persistent storage.
    /// Incremented by `add_approved_spender`. Used by
    /// `estimated_storage_footprint`.
    SpenderCount,
    /// Monotonically increasing schema version stored in instance storage.
    /// 0  = pre-versioning (no Version key present).
    /// 1  = current schema (this build).
    Version,
}
