//! Shared types for the RewardPool contract.

use soroban_sdk::{contracttype, Address};

/// Storage keys used by the RewardPool contract.
///
/// | Variant | Storage tier | Type | Description |
/// |---------|-------------|------|-------------|
/// | `Admin` | Instance | `Address` | Protocol admin address |
/// | `Token` | Instance | `Address` | SAC reward-token address |
/// | `Spender(Address)` | Persistent | `bool` | Approved spender flag |
/// | `IsPaused` | Instance | `bool` | Global pause circuit-breaker |
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Instance storage key for the protocol admin [`Address`].
    Admin,
    /// Instance storage key for the reward SAC token [`Address`].
    Token,
    /// Persistent storage key indicating whether a given [`Address`] is
    /// an approved spender permitted to call `distribute_reward`.
    Spender(Address),
    /// Instance storage key for the global pause flag.
    /// When `true`, `distribute_reward` panics with `"Contract is paused"`.
    IsPaused,
}
