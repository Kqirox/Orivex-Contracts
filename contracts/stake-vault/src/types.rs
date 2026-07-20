//! Shared types for the StakeVault contract.

use soroban_sdk::{contracttype, Address};

/// Stake record for a single user, stored in persistent storage.
///
/// Both fields are updated atomically on every `stake` call:
/// * `amount` accumulates across multiple calls (multi-stake is supported).
/// * `lock_timestamp` is **reset** on every `stake` call, restarting the
///   one-week lock window regardless of any prior stake.
///
/// # Invariants
///
/// * `amount` is always `> 0` while the record exists in storage. The slot
///   is deleted by `unstake`, so a missing key always means no active stake.
/// * `lock_timestamp` is a Soroban ledger Unix timestamp in seconds.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakeInfo {
    /// Total tokens currently locked by this user.
    pub amount: i128,
    /// Ledger timestamp (Unix seconds) of the most recent `stake` call.
    /// The lock expires at `lock_timestamp + DEFAULT_LOCK_PERIOD_SECONDS`.
    pub lock_timestamp: u64,
}

/// Storage keys used by the StakeVault contract.
///
/// | Variant | Storage tier | Type | Description |
/// |---------|-------------|------|-------------|
/// | `Admin` | Instance | `Address` | Protocol admin address |
/// | `Token` | Instance | `Address` | Staking token address |
/// | `UserStake(Address)` | Persistent | [`StakeInfo`] | Per-user stake record |
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Instance storage key for the protocol admin [`Address`].
    Admin,
    /// Instance storage key for the staking token [`Address`].
    Token,
    /// Persistent storage key for the [`StakeInfo`] of a given user.
    UserStake(Address),
}
