//! Invariant tests for the BadgeNFT contract.
//!
//! These tests verify two classes of guarantees after every public method:
//!
//! 1. **Event cardinality** — exactly the documented number of events is emitted.
//! 2. **Storage consistency** — the persistent storage shape matches expectations
//!    (e.g. UserBadges slot exists iff a badge was minted; no orphaned keys).
//!
//! Each test is named `inv_<method>_<what_is_checked>`.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, Env,
};

use crate::{
    types::{Badge, DataKey},
    BadgeNFT, BadgeNFTClient,
};

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Assert the total number of events recorded in `env` equals `expected`.
///
/// Fails with a clear message listing the actual event count so test output
/// immediately shows cardinality drift when a new method adds/removes an emit.
macro_rules! assert_event_count {
    ($env:expr, $expected:expr) => {{
        let actual = $env.events().all().len();
        assert_eq!(
            actual, $expected,
            "event-count invariant violated: expected {} event(s), got {}",
            $expected, actual
        );
    }};
}

/// Check that `DataKey::UserBadges(addr)` exists in persistent storage and
/// that the stored vector has exactly `expected_len` entries.
fn assert_user_badges_len(env: &Env, contract: &Address, learner: &Address, expected_len: u32) {
    env.as_contract(contract, || {
        let badges: soroban_sdk::Vec<Badge> = env
            .storage()
            .persistent()
            .get(&DataKey::UserBadges(learner.clone()))
            .unwrap_or_else(|| soroban_sdk::Vec::new(env));
        assert_eq!(
            badges.len(),
            expected_len,
            "storage invariant violated: UserBadges({:?}) expected len={}, got {}",
            learner,
            expected_len,
            badges.len()
        );
    });
}

/// Assert that `DataKey::Admin` exists in instance storage and equals `expected`.
fn assert_admin_stored(env: &Env, contract: &Address, expected: &Address) {
    env.as_contract(contract, || {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("invariant violated: DataKey::Admin must be set after initialize");
        assert_eq!(
            stored, *expected,
            "storage invariant violated: Admin address mismatch"
        );
    });
}

/// Assert that `DataKey::UserBadges(addr)` does NOT exist in persistent storage
/// (used to verify no orphaned key is written for zero-badge learners).
fn assert_no_user_badges_key(env: &Env, contract: &Address, learner: &Address) {
    env.as_contract(contract, || {
        // An empty Vec written to storage would still be a present key.
        // We accept a missing key OR an empty vec as "no orphan".
        let vec: Option<soroban_sdk::Vec<Badge>> = env
            .storage()
            .persistent()
            .get(&DataKey::UserBadges(learner.clone()));
        if let Some(v) = vec {
            assert_eq!(
                v.len(),
                0,
                "storage invariant violated: expected no UserBadges key for {:?}, \
                 but found {} badge(s)",
                learner,
                v.len()
            );
        }
    });
}

fn setup() -> (Env, BadgeNFTClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(BadgeNFT, ());
    let client = BadgeNFTClient::new(&env, &id);
    (env, client)
}

// ── initialize ────────────────────────────────────────────────────────────────

/// `initialize` must emit **0** events and store the admin address.
#[test]
fn inv_initialize_emits_no_events() {
    let (env, client) = setup();
    let admin = Address::generate(&env);

    client.initialize(&admin);

    assert_event_count!(env, 0);
}

/// After `initialize`, `DataKey::Admin` must be present and correct.
#[test]
fn inv_initialize_stores_admin() {
    let (env, client) = setup();
    let admin = Address::generate(&env);

    client.initialize(&admin);

    assert_admin_stored(&env, &client.address, &admin);
}

/// After `initialize`, no `UserBadges` slot should exist for any address
/// (no orphaned storage from the init path).
#[test]
fn inv_initialize_no_orphan_user_badges() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let random = Address::generate(&env);

    client.initialize(&admin);

    assert_no_user_badges_key(&env, &client.address, &random);
}

// ── mint_badge ────────────────────────────────────────────────────────────────

/// `mint_badge` must emit exactly **1** event (`BadgeMinted`).
#[test]
fn inv_mint_badge_emits_exactly_one_event() {
    let (env, client) = setup();
    let registry = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&registry);
    client.mint_badge(&registry, &learner, &1);

    assert_event_count!(env, 1);
}

/// After `mint_badge`, `UserBadges(learner)` must contain exactly 1 entry.
#[test]
fn inv_mint_badge_creates_user_badges_slot() {
    let (env, client) = setup();
    let registry = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&registry);
    client.mint_badge(&registry, &learner, &42);

    assert_user_badges_len(&env, &client.address, &learner, 1);
}

/// Minting N badges for the same learner → `UserBadges` len == N and
/// exactly N events are emitted (one per mint, no extras).
#[test]
fn inv_mint_badge_event_count_scales_linearly() {
    let (env, client) = setup();
    let registry = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&registry);
    client.mint_badge(&registry, &learner, &1);
    client.mint_badge(&registry, &learner, &2);
    client.mint_badge(&registry, &learner, &3);

    assert_event_count!(env, 3);
    assert_user_badges_len(&env, &client.address, &learner, 3);
}

/// Minting for learner A must not create a `UserBadges` slot for learner B.
#[test]
fn inv_mint_badge_no_cross_learner_storage_leak() {
    let (env, client) = setup();
    let registry = Address::generate(&env);
    let learner_a = Address::generate(&env);
    let learner_b = Address::generate(&env);

    client.initialize(&registry);
    client.mint_badge(&registry, &learner_a, &7);

    // learner_b should have no storage slot
    assert_no_user_badges_key(&env, &client.address, &learner_b);
    // learner_a should have exactly 1
    assert_user_badges_len(&env, &client.address, &learner_a, 1);
}

/// After a successful mint, `DataKey::Admin` must still hold the original admin
/// (mint must not mutate admin storage).
#[test]
fn inv_mint_badge_admin_unchanged() {
    let (env, client) = setup();
    let registry = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&registry);
    client.mint_badge(&registry, &learner, &5);

    assert_admin_stored(&env, &client.address, &registry);
}

// ── revoke_badge ──────────────────────────────────────────────────────────────

/// `revoke_badge` on an existing badge must emit exactly **1** event (`BadgeRevoked`).
#[test]
fn inv_revoke_badge_emits_exactly_one_event() {
    let (env, client) = setup();
    let registry = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&registry);
    client.mint_badge(&registry, &learner, &10);
    env.events().all(); // drain — we only care about revoke's events

    // Re-read after drain: count from fresh
    let events_before = env.events().all().len();
    client.revoke_badge(&registry, &learner, &10);
    let events_after = env.events().all().len();

    assert_eq!(
        events_after - events_before,
        1,
        "revoke_badge must emit exactly 1 event; delta={}",
        events_after - events_before
    );
}

/// After `revoke_badge`, the learner's badge vector must shrink by exactly 1.
#[test]
fn inv_revoke_badge_shrinks_user_badges() {
    let (env, client) = setup();
    let registry = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&registry);
    client.mint_badge(&registry, &learner, &1);
    client.mint_badge(&registry, &learner, &2);

    assert_user_badges_len(&env, &client.address, &learner, 2);

    client.revoke_badge(&registry, &learner, &1);

    assert_user_badges_len(&env, &client.address, &learner, 1);
}

/// Revoking a non-existent badge (no-op path) must emit **0** events.
#[test]
fn inv_revoke_badge_nonexistent_emits_no_event() {
    let (env, client) = setup();
    let registry = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&registry);
    client.mint_badge(&registry, &learner, &1);

    // revoke a course_id the learner does not hold
    let before = env.events().all().len();
    client.revoke_badge(&registry, &learner, &999);
    let after = env.events().all().len();

    assert_eq!(
        after - before,
        0,
        "revoke_badge on non-existent badge must emit 0 events; delta={}",
        after - before
    );
}

/// After revoking all badges, the `UserBadges` vector must be empty (len 0),
/// not absent with orphaned data.
#[test]
fn inv_revoke_badge_all_leaves_empty_vec_not_stale_data() {
    let (env, client) = setup();
    let registry = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&registry);
    client.mint_badge(&registry, &learner, &1);
    client.revoke_badge(&registry, &learner, &1);

    assert_user_badges_len(&env, &client.address, &learner, 0);
}

// ── get_badges / get_badge_count / has_badge (pure reads) ────────────────────

/// Pure read methods must not emit any events.
#[test]
fn inv_read_methods_emit_no_events() {
    let (env, client) = setup();
    let registry = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&registry);
    client.mint_badge(&registry, &learner, &1);

    // Drain events from setup + mint
    let baseline = env.events().all().len();

    let _ = client.get_badges(&learner);
    let _ = client.get_badge_count(&learner);
    let _ = client.has_badge(&learner, &1);
    let _ = client.has_badge(&learner, &99);

    assert_eq!(
        env.events().all().len(),
        baseline,
        "read methods must not emit any events"
    );
}

/// `get_badge_count` must always agree with `get_badges().len()`.
#[test]
fn inv_get_badge_count_consistent_with_get_badges() {
    let (env, client) = setup();
    let registry = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&registry);

    // 0 badges
    assert_eq!(client.get_badge_count(&learner), client.get_badges(&learner).len());

    client.mint_badge(&registry, &learner, &1);
    assert_eq!(client.get_badge_count(&learner), client.get_badges(&learner).len());

    client.mint_badge(&registry, &learner, &2);
    assert_eq!(client.get_badge_count(&learner), client.get_badges(&learner).len());

    client.revoke_badge(&registry, &learner, &1);
    assert_eq!(client.get_badge_count(&learner), client.get_badges(&learner).len());
}

/// `has_badge` must return `true` iff the badge appears in `get_badges()`.
#[test]
fn inv_has_badge_consistent_with_get_badges() {
    let (env, client) = setup();
    let registry = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&registry);
    client.mint_badge(&registry, &learner, &10);
    client.mint_badge(&registry, &learner, &20);

    let badges = client.get_badges(&learner);
    let ids: soroban_sdk::Vec<u32> = {
        let mut v = soroban_sdk::Vec::new(&env);
        for b in badges.iter() {
            v.push_back(b.course_id);
        }
        v
    };

    assert!(client.has_badge(&learner, &10));
    assert!(ids.contains(10));

    assert!(client.has_badge(&learner, &20));
    assert!(ids.contains(20));

    assert!(!client.has_badge(&learner, &30));
    assert!(!ids.contains(30));
}
