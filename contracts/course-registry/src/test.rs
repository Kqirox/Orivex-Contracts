#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, BytesN, Env,
};

use crate::{CourseRegistry, CourseRegistryClient};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup() -> (Env, CourseRegistryClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    // Fixed: Passing the contract type first, and empty constructor args second
    let contract_id = env.register(CourseRegistry, ());

    let client = CourseRegistryClient::new(&env, &contract_id);
    (env, client)
}

fn dummy_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[1u8; 32])
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_create_course_returns_id_one() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);

    client.initialize(&admin);

    let id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));
    assert_eq!(id, 1);
}

#[test]
fn test_course_count_increments() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let hash = dummy_hash(&env);

    client.initialize(&admin);

    assert_eq!(client.course_count(), 0);
    client.create_course(&admin, &instructor, &2, &hash);
    assert_eq!(client.course_count(), 1);
    client.create_course(&admin, &instructor, &5, &hash);
    assert_eq!(client.course_count(), 2);
}

#[test]
#[should_panic(expected = "total_modules must be greater than 0")]
fn test_zero_modules_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);

    client.initialize(&admin);
    client.create_course(&admin, &instructor, &0, &dummy_hash(&env));
}

#[test]
#[should_panic(expected = "Unauthorized: Caller is not the protocol admin")]
fn test_unauthorized_admin_panics() {
    let (env, client) = setup();
    let true_admin = Address::generate(&env);
    let fake_admin = Address::generate(&env);
    let instructor = Address::generate(&env);

    client.initialize(&true_admin);

    // Fails because fake_admin does not match true_admin
    client.create_course(&fake_admin, &instructor, &3, &dummy_hash(&env));
}

#[test]
fn test_course_created_event_emitted() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let hash = dummy_hash(&env);

    client.initialize(&admin);
    client.create_course(&admin, &instructor, &4, &hash);

    // Verify exactly one contract event was published via the macro.
    assert_eq!(env.events().all().len(), 1);
}

#[test]
fn test_update_metadata_success() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let hash = dummy_hash(&env);
    let new_hash = BytesN::from_array(&env, &[2u8; 32]);

    client.initialize(&admin);
    client.create_course(&admin, &instructor, &3, &hash);
    client.update_metadata(&1, &new_hash);
}

#[test]
#[should_panic(expected = "Course not found")]
fn test_update_nonexistent_course() {
    let (env, client) = setup();
    let admin = Address::generate(&env);

    client.initialize(&admin);
    client.update_metadata(&99, &dummy_hash(&env));
}

#[test]
fn test_update_metadata_emits_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let hash = dummy_hash(&env);
    let new_hash = BytesN::from_array(&env, &[2u8; 32]);

    client.initialize(&admin);
    client.create_course(&admin, &instructor, &3, &hash);
    client.update_metadata(&1, &new_hash);

    // events().all() returns events from the most recent invocation
    assert_eq!(env.events().all().len(), 1);
}

#[test]
fn test_update_metadata_multiple_times() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let hash = dummy_hash(&env);
    let hash_v2 = BytesN::from_array(&env, &[2u8; 32]);
    let hash_v3 = BytesN::from_array(&env, &[3u8; 32]);

    client.initialize(&admin);
    client.create_course(&admin, &instructor, &3, &hash);
    client.update_metadata(&1, &hash_v2);
    client.update_metadata(&1, &hash_v3);
}

// ── complete_module Tests ─────────────────────────────────────────────────────

#[test]
fn test_complete_module_increments_progress() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let course_id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));

    // Complete first module
    client.complete_module(&admin, &learner, &course_id);
}

#[test]
fn test_complete_module_emits_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let course_id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));

    client.complete_module(&admin, &learner, &course_id);

    // Verify event was emitted
    assert_eq!(env.events().all().len(), 1);
}

#[test]
fn test_complete_module_multiple_times() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let course_id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));

    // Complete all three modules
    client.complete_module(&admin, &learner, &course_id);
    client.complete_module(&admin, &learner, &course_id);
    client.complete_module(&admin, &learner, &course_id);
}

#[test]
#[should_panic(expected = "Course already completed")]
fn test_complete_module_exceeds_total_modules() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let course_id = client.create_course(&admin, &instructor, &2, &dummy_hash(&env));

    // Complete both modules
    client.complete_module(&admin, &learner, &course_id);
    client.complete_module(&admin, &learner, &course_id);

    // This should panic - trying to complete a third module when only 2 exist
    client.complete_module(&admin, &learner, &course_id);
}

#[test]
#[should_panic(expected = "Unauthorized: Caller is not the protocol admin")]
fn test_complete_module_unauthorized_verifier() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let fake_verifier = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let course_id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));

    // Should fail - fake_verifier is not the admin
    client.complete_module(&fake_verifier, &learner, &course_id);
}

#[test]
#[should_panic(expected = "Course not found")]
fn test_complete_module_nonexistent_course() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);

    // Should fail - course 99 doesn't exist
    client.complete_module(&admin, &learner, &99);
}

#[test]
fn test_complete_module_different_learners() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner1 = Address::generate(&env);
    let learner2 = Address::generate(&env);

    client.initialize(&admin);
    let course_id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));

    // Both learners can progress independently
    client.complete_module(&admin, &learner1, &course_id);
    client.complete_module(&admin, &learner2, &course_id);
    client.complete_module(&admin, &learner1, &course_id);
}

#[test]
fn test_get_progress_returns_zero_initially() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let course_id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));

    // Progress should be 0 before any modules are completed
    assert_eq!(client.get_progress(&learner, &course_id), 0);
}

#[test]
fn test_get_progress_tracks_completion() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    client.initialize(&admin);
    let course_id = client.create_course(&admin, &instructor, &3, &dummy_hash(&env));

    assert_eq!(client.get_progress(&learner, &course_id), 0);

    client.complete_module(&admin, &learner, &course_id);
    assert_eq!(client.get_progress(&learner, &course_id), 1);

    client.complete_module(&admin, &learner, &course_id);
    assert_eq!(client.get_progress(&learner, &course_id), 2);

    client.complete_module(&admin, &learner, &course_id);
    assert_eq!(client.get_progress(&learner, &course_id), 3);
}
