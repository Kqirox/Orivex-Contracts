//! Shared types for the BadgeNFT contract.

use soroban_sdk::{contracttype, Address};

/// An individual soulbound badge awarded to a learner upon course completion.
///
/// Each `Badge` is stored inside the learner's `Vec<Badge>` under
/// `DataKey::UserBadges(learner_address)` in persistent storage.
///
/// # Invariants
///
/// * `(learner_address, course_id)` pairs are unique — the `mint_badge`
///   function enforces this before insertion.
/// * `minted_at` is the Soroban ledger timestamp at the moment of minting.
///   A value of `0` is used only in tests or when the ledger timestamp is
///   unavailable (see [`BADGE_MINTED_AT_DEFAULT`](crate::BADGE_MINTED_AT_DEFAULT)).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Badge {
    /// On-chain identifier of the course this badge represents.
    pub course_id: u32,
    /// Ledger timestamp (Unix seconds) at the time the badge was minted.
    pub minted_at: u64,
}

/// Storage keys used by the BadgeNFT contract.
///
/// Variants map to specific Soroban storage tiers:
///
/// | Variant | Storage tier | Notes |
/// |---------|-------------|-------|
/// | `Admin` | Instance | Single authorized minter address |
/// | `UserBadges(Address)` | Persistent | Badge vector keyed by learner |
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Stores the authorized registry / admin [`Address`].
    Admin,
    /// Stores the `Vec<Badge>` for the given learner [`Address`].
    UserBadges(Address),
}
