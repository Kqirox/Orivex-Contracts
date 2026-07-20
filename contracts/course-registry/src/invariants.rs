//! Invariant tests for the CourseRegistry contract.
//!
//! For every public method these tests assert:
//!
//! 1. **Event cardinality** — exactly the documented number of events is emitted.
//! 2. **Storage consistency** — `Course`, `Progress`, `CourseCount`, and
//!    wired-address slots are in the correct state after the call.
//! 3. **Cross-contract invariants** — BadgeNFT badge minted iff final module
//!    completed; no badge on intermediate modules.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, BytesN, Env,
};

use crate::{
    types::{Course, DataKey},
    CourseRegistry, CourseRegistryClient,
};
use badge_nft::{BadgeNFT, BadgeNFTClient};

// ── Shared helpers ────────────────────────────────────────────────────────────

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

fn events_since(env: &Env, baseline: usize) -> usize {
    env.events().all().len() - baseline
}

fn setup() -> (Env, CourseRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(CourseRegistry, ());
    let client = CourseRegistryClient::new(&env, &id);
    (env, client)
}

fn dummy_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0xffu8; 32])
}

fn assert_course_exists(env: &Env, contract: &Address, id: u32) -> Course {
    env.as_contract(contract, || {
        env.storage()
            .persistent()
            .get(&DataKey::Course(id))
            .expect("invariant: DataKey::Course(id) must exist after create_course")
    })
}

fn assert_course_count(env: &Env, contract: &Address, expected: u32) {
    env.as_contract(contract, || {
        let stored: u32 = env
            .storage()
            .instance()
            .get(&DataKey::CourseCount)
            .unwrap_or(0);
        assert_eq!(
            stored, expected,
            "CourseCount invariant: expected={}, got={}",
            expected, stored
        );
    });
}

fn assert_progress(env: &Env, contract: &Address, learner: &Address, course_id: u32, expected: u32) {
    env.as_contract(contract, || {
        let progress: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Progress(learner.clone(), course_id))
            .unwrap_or(0);
        assert_eq!(
            progress, expected,
            "Progress({:?}, {}) expected={}, got={}",
            learner, course_id, expected, progress
        );
    });
}

fn assert_admin_stored(env: &Env, contract: &Address, expected: &Address) {
    env.as_contract(contract, || {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("invariant: DataKey::Admin must be set");
        assert_eq!(stored, *expected);
    });
}

// ── initialize ────────────────────────────────────────────────────────────────

/// `initialize` must emit **0** events.
#[test]
fn inv_initialize_emits_no_events() {
    let (env, client) = setup();
    let admin = Address::generate(&env);

    client.initialize(&admin);

    assert_event_count!(env, 0);
}

/// After `initialize`, `DataKey::Admin` must be stored correctly.
#[test]
fn inv_initialize_stores_admin() {
    let (env, client) = setup();
    let admin = Address::generate(&env);

    client.initialize(&admin);

    assert_admin_stored(&env, &client.address, &admin);
}

/// After `initialize`, `CourseCount` must be **0**.
#[test]
fn inv_initialize_course_count_is_zero() {
    let (env, client) = setup();
    let admin = Address::generate(&env);

    client.initialize(&admin);

    assert_course_count(&env, &client.address, 0);
}

// ── create_course ─────────────────────────────────────────────────────────────

/// `create_course` must emit exactly **1** event (`CourseCreated`).
#[test]
fn inv_create_course_emits_exactly_one_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);

    client.initialize(&admin);
    let baseline = env.events().all().len();

    client.create_course(&admin, &instructor, &3, &dummy_hash(&env));

    assert_eq!(events_since(&env, baseline), 1,
        "create_course must emit exactly 1 event");
}

/// After `create_course`, `DataKey::Course(id)` must exist and match inputs.
#[test]
fn inv_create_course_slot_matches_inputs() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let hash = dummy_hash(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &5, &hash);

    let course = assert_course_exists(&env, &client.address, id);
    assert_eq!(course.instructor, instructor);
    assert_eq!(course.total_modules, 5);
    assert_eq!(course.metadata_hash, hash);
    assert!(course.active, "new course must be active");
}

/// `CourseCount` must increment by 1 per `create_course`.
#[test]
fn inv_create_course_increments_count() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let hash = dummy_hash(&env);

    client.initialize(&admin);
    assert_course_count(&env, &client.address, 0);

    client.create_course(&admin, &instructor, &3, &hash);
    assert_course_count(&env, &client.address, 1);

    client.create_course(&admin, &instructor, &4, &hash);
    assert_course_count(&env, &client.address, 2);
}

/// Creating N courses must emit exactly N events.
#[test]
fn inv_create_course_event_count_linear() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let hash = dummy_hash(&env);

    client.initialize(&admin);
    let baseline = env.events().all().len();

    for i in 1u32..=4 {
        client.create_course(&admin, &instructor, &i, &hash);
    }

    assert_eq!(events_since(&env, baseline), 4);
}

// ── update_metadata ───────────────────────────────────────────────────────────

/// `update_metadata` must emit exactly **1** event (`MetadataUpdated`).
#[test]
fn inv_update_metadata_emits_exactly_one_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));
    let baseline = env.events().all().len();

    let new_hash = BytesN::from_array(&env, &[0xaau8; 32]);
    client.update_metadata(&id, &new_hash);

    assert_eq!(events_since(&env, baseline), 1,
        "update_metadata must emit exactly 1 event");
}

/// After `update_metadata`, `Course.metadata_hash` must equal the new hash.
#[test]
fn inv_update_metadata_hash_stored() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));
    let new_hash = BytesN::from_array(&env, &[0xbbu8; 32]);

    client.update_metadata(&id, &new_hash);

    let course = assert_course_exists(&env, &client.address, id);
    assert_eq!(course.metadata_hash, new_hash,
        "metadata_hash must be updated in storage");
}

/// `update_metadata` must not change `instructor`, `total_modules`, or `active`.
#[test]
fn inv_update_metadata_other_fields_unchanged() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &7, &dummy_hash(&env));
    let new_hash = BytesN::from_array(&env, &[0xccu8; 32]);

    client.update_metadata(&id, &new_hash);

    let course = assert_course_exists(&env, &client.address, id);
    assert_eq!(course.instructor, instructor);
    assert_eq!(course.total_modules, 7);
    assert!(course.active);
}

// ── enroll ────────────────────────────────────────────────────────────────────

/// `enroll` must emit **0** events (no event is defined for enrollment).
#[test]
fn inv_enroll_emits_no_events() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));
    let baseline = env.events().all().len();

    client.enroll(&learner, &id);

    assert_eq!(events_since(&env, baseline), 0,
        "enroll must emit 0 events");
}

/// After `enroll`, `Progress(learner, course_id)` must be **0**.
#[test]
fn inv_enroll_creates_zero_progress_slot() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));
    client.enroll(&learner, &id);

    assert_progress(&env, &client.address, &learner, id, 0);
}

/// Enrolling learner A must not create a progress slot for learner B.
#[test]
fn inv_enroll_no_cross_learner_progress_leak() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner_a = Address::generate(&env);
    let learner_b = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));
    client.enroll(&learner_a, &id);

    // learner_b should have no progress slot (unwrap_or returns 0 but key absent)
    env.as_contract(&client.address, || {
        let has = env
            .storage()
            .persistent()
            .has(&DataKey::Progress(learner_b.clone(), id));
        assert!(!has, "enroll for A must not create progress slot for B");
    });
}

// ── set_course_status ─────────────────────────────────────────────────────────

/// `set_course_status` must emit exactly **1** event (`CourseStatusChanged`).
#[test]
fn inv_set_course_status_emits_exactly_one_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));
    let baseline = env.events().all().len();

    client.set_course_status(&admin, &id, &false);

    assert_eq!(events_since(&env, baseline), 1,
        "set_course_status must emit exactly 1 event");
}

/// After `set_course_status(false)`, `Course.active` must be `false`
/// and all other course fields must be unchanged.
#[test]
fn inv_set_course_status_only_active_changes() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let hash = dummy_hash(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &4, &hash);
    client.set_course_status(&admin, &id, &false);

    let course = assert_course_exists(&env, &client.address, id);
    assert!(!course.active);
    assert_eq!(course.instructor, instructor);
    assert_eq!(course.total_modules, 4);
    assert_eq!(course.metadata_hash, hash);
}

// ── transfer_ownership ────────────────────────────────────────────────────────

/// `transfer_ownership` must emit exactly **1** event (`OwnershipTransferred`).
#[test]
fn inv_transfer_ownership_emits_exactly_one_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let new_instructor = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));
    let baseline = env.events().all().len();

    client.transfer_ownership(&instructor, &new_instructor, &id);

    assert_eq!(events_since(&env, baseline), 1,
        "transfer_ownership must emit exactly 1 event");
}

/// After `transfer_ownership`, `Course.instructor` must equal `new_instructor`.
#[test]
fn inv_transfer_ownership_updates_instructor() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let new_instructor = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));
    client.transfer_ownership(&instructor, &new_instructor, &id);

    let course = assert_course_exists(&env, &client.address, id);
    assert_eq!(course.instructor, new_instructor);
}

/// `transfer_ownership` must not change any field other than `instructor`.
#[test]
fn inv_transfer_ownership_other_fields_unchanged() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let new_instructor = Address::generate(&env);
    let hash = dummy_hash(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &6, &hash);
    client.transfer_ownership(&instructor, &new_instructor, &id);

    let course = assert_course_exists(&env, &client.address, id);
    assert_eq!(course.total_modules, 6);
    assert_eq!(course.metadata_hash, hash);
    assert!(course.active);
}

// ── complete_module ───────────────────────────────────────────────────────────

/// Non-final `complete_module` must emit exactly **1** event (`ModuleCompleted`).
#[test]
fn inv_complete_module_non_final_emits_one_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));
    let baseline = env.events().all().len();

    client.complete_module(&admin, &learner, &id); // module 1 of 3

    assert_eq!(events_since(&env, baseline), 1,
        "non-final complete_module must emit exactly 1 event (ModuleCompleted)");
}

/// Final `complete_module` with no integrations wired emits exactly **1** event.
#[test]
fn inv_complete_module_final_no_integrations_emits_one_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &1, &dummy_hash(&env));
    let baseline = env.events().all().len();

    client.complete_module(&admin, &learner, &id); // final module, no badge/reward wired

    assert_eq!(events_since(&env, baseline), 1,
        "final complete_module (no integrations) must emit exactly 1 event");
}

/// `complete_module` must increment `Progress` by exactly 1 each time.
#[test]
fn inv_complete_module_progress_increments() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));

    client.complete_module(&admin, &learner, &id);
    assert_progress(&env, &client.address, &learner, id, 1);

    client.complete_module(&admin, &learner, &id);
    assert_progress(&env, &client.address, &learner, id, 2);

    client.complete_module(&admin, &learner, &id);
    assert_progress(&env, &client.address, &learner, id, 3);
}

/// Progress for learner A must not change when learner B completes a module.
#[test]
fn inv_complete_module_progress_isolated_per_learner() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner_a = Address::generate(&env);
    let learner_b = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &5, &dummy_hash(&env));

    client.complete_module(&admin, &learner_a, &id);
    client.complete_module(&admin, &learner_a, &id);

    assert_progress(&env, &client.address, &learner_a, id, 2);
    // B has never been touched
    assert_progress(&env, &client.address, &learner_b, id, 0);
}

/// On final module with BadgeNFT wired: exactly **1** badge minted for the learner.
#[test]
fn inv_complete_module_final_badge_minted_exactly_once() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &2, &dummy_hash(&env));

    let badge_id = env.register(BadgeNFT, ());
    let badge_client = BadgeNFTClient::new(&env, &badge_id);
    badge_client.initialize(&client.address);
    client.set_badge_nft_address(&admin, &badge_client.address);

    // Module 1 — no badge
    client.complete_module(&admin, &learner, &id);
    assert_eq!(badge_client.get_badge_count(&learner), 0,
        "badge must not be minted before final module");

    // Module 2 (final) — exactly 1 badge
    client.complete_module(&admin, &learner, &id);
    assert_eq!(badge_client.get_badge_count(&learner), 1,
        "exactly 1 badge must be minted on final module");
    assert!(badge_client.has_badge(&learner, &id),
        "badge must be for the correct course");
}

/// Completing the same final module for N learners → each gets exactly 1 badge, no cross-contamination.
#[test]
fn inv_complete_module_badge_isolated_per_learner() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner_a = Address::generate(&env);
    let learner_b = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &1, &dummy_hash(&env));

    let badge_id = env.register(BadgeNFT, ());
    let badge_client = BadgeNFTClient::new(&env, &badge_id);
    badge_client.initialize(&client.address);
    client.set_badge_nft_address(&admin, &badge_client.address);

    client.complete_module(&admin, &learner_a, &id);
    client.complete_module(&admin, &learner_b, &id);

    assert_eq!(badge_client.get_badge_count(&learner_a), 1);
    assert_eq!(badge_client.get_badge_count(&learner_b), 1);
    assert!(!badge_client.has_badge(&learner_a, &(id + 99)),
        "badge must be keyed to the correct course only");
}

// ── set_badge_nft_address / set_reward_pool_address (admin config) ────────────

/// `set_badge_nft_address` must emit **0** events and persist the address.
#[test]
fn inv_set_badge_nft_address_emits_no_events_stores_address() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let badge_addr = Address::generate(&env);

    client.initialize(&admin);
    let baseline = env.events().all().len();

    client.set_badge_nft_address(&admin, &badge_addr);

    assert_eq!(events_since(&env, baseline), 0,
        "set_badge_nft_address must emit 0 events");

    env.as_contract(&client.address, || {
        let stored: Option<Address> = env
            .storage()
            .instance()
            .get(&DataKey::BadgeNftAddress);
        assert_eq!(stored, Some(badge_addr.clone()),
            "BadgeNftAddress must be stored after set_badge_nft_address");
    });
}

/// `set_reward_pool_address` must emit **0** events and persist the address.
#[test]
fn inv_set_reward_pool_address_emits_no_events_stores_address() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let pool_addr = Address::generate(&env);

    client.initialize(&admin);
    let baseline = env.events().all().len();

    client.set_reward_pool_address(&admin, &pool_addr);

    assert_eq!(events_since(&env, baseline), 0,
        "set_reward_pool_address must emit 0 events");

    env.as_contract(&client.address, || {
        let stored: Option<Address> = env
            .storage()
            .instance()
            .get(&DataKey::RewardPoolAddress);
        assert_eq!(stored, Some(pool_addr.clone()),
            "RewardPoolAddress must be stored after set_reward_pool_address");
    });
}

// ── course_count / get_course / get_progress / is_course_finished (pure reads) ─

/// Pure read methods must not emit any events.
#[test]
fn inv_read_methods_emit_no_events() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));
    client.enroll(&learner, &id);

    let baseline = env.events().all().len();

    let _ = client.course_count();
    let _ = client.get_course(&id);
    let _ = client.get_progress(&learner, &id);
    let _ = client.is_course_finished(&learner, &id);

    assert_eq!(events_since(&env, baseline), 0,
        "read methods must not emit any events");
}

/// `course_count()` must always equal `CourseCount` in instance storage.
#[test]
fn inv_course_count_consistent_with_storage() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let hash = dummy_hash(&env);

    client.initialize(&admin);
    assert_eq!(client.course_count(), 0);
    assert_course_count(&env, &client.address, 0);

    client.create_course(&admin, &instructor, &3, &hash);
    assert_eq!(client.course_count(), 1);
    assert_course_count(&env, &client.address, 1);

    client.create_course(&admin, &instructor, &5, &hash);
    assert_eq!(client.course_count(), 2);
    assert_course_count(&env, &client.address, 2);
}

/// `is_course_finished` must return `true` iff `get_progress >= total_modules`.
#[test]
fn inv_is_course_finished_consistent_with_progress() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let id = client.create_course(&admin, &instructor, &2, &dummy_hash(&env));

    assert!(!client.is_course_finished(&learner, &id));

    client.complete_module(&admin, &learner, &id);
    assert_eq!(client.get_progress(&learner, &id), 1);
    assert!(!client.is_course_finished(&learner, &id));

    client.complete_module(&admin, &learner, &id);
    assert_eq!(client.get_progress(&learner, &id), 2);
    assert!(client.is_course_finished(&learner, &id));
}
