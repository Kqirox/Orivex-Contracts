#![no_std]
//! Shared TTL constants and bump-on-touch helpers for the Orivex protocol.
//!
//! # Storage TTL Semantics
//!
//! Soroban persistent storage entries have a live-until ledger (TTL) after
//! which they expire and are silently removed. The Orivex protocol follows a
//! **bump-on-touch** policy: every function that reads **or** writes a
//! persistent key must extend its TTL so that active protocol data never
//! vanishes between user interactions.
//!
//! ## Constants
//! | Constant | Value | Approx. real time |
//! |---|---|---|
//! | `LEDGER_BUMP_PERSISTENT` | 535,000 ledgers | ≈ 30 days at 5 s/ledger |
//! | `LEDGER_THRESHOLD_PERSISTENT` | 517,000 ledgers | Bump triggers when TTL < this |
//!
//! The threshold is set 18,000 ledgers (≈25 h) below the bump target so that
//! storage is only extended when it is genuinely close to expiry, which keeps
//! per-call fee overhead minimal.
//!
//! ## Usage
//! ```ignore
//! use orivex_common::bump_persistent;
//!
//! // after any persistent read or write:
//! bump_persistent(&env, &DataKey::Course(id));
//! ```

use soroban_sdk::Env;

/// Target TTL added when bumping a persistent storage entry.
///
/// 535,000 ledgers ≈ 30 days at the Soroban default of 5 seconds per ledger.
pub const LEDGER_BUMP_PERSISTENT: u32 = 535_000;

/// Minimum remaining TTL before a bump is applied.
///
/// Set 18,000 ledgers (≈ 25 hours) below `LEDGER_BUMP_PERSISTENT` so that
/// redundant bumps on hot entries are cheap — the host only performs a write
/// if the current TTL is below this threshold.
pub const LEDGER_THRESHOLD_PERSISTENT: u32 = 517_000;

/// Extend the TTL of a persistent storage key if its remaining live-until
/// ledger falls below [`LEDGER_THRESHOLD_PERSISTENT`].
///
/// Call this after every persistent `get`, `set`, or `has` that touches a
/// key whose data must outlive a single session.  The function is a no-op
/// when the key's TTL already exceeds the threshold, so calling it
/// unconditionally is safe and incurs no extra fee unless an actual bump is
/// required.
///
/// # Arguments
/// * `env`  – The Soroban [`Env`] for the current invocation.
/// * `key`  – A reference to the storage key whose TTL should be extended.
///            The key type must implement `soroban_sdk::Val` (i.e. be a
///            `#[contracttype]`).
///
/// # Example
/// ```ignore
/// bump_persistent(&env, &DataKey::UserBadges(learner.clone()));
/// ```
pub fn bump_persistent<K>(env: &Env, key: &K)
where
    K: soroban_sdk::Val,
{
    env.storage()
        .persistent()
        .extend_ttl(key, LEDGER_THRESHOLD_PERSISTENT, LEDGER_BUMP_PERSISTENT);
}

#[cfg(test)]
mod ttl_audit_test;

