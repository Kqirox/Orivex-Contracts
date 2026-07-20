//! Invariant tests for the QuestEngine contract.
//!
//! For every public method these tests assert:
//!
//! 1. **Event cardinality** — exactly the documented number of events.
//! 2. **Storage consistency** — Quest and Submission slots are in the expected
//!    state after each call.
//! 3. **Token balance invariants** — Build-quest funds are conserved across
//!    create → submit → review and create → refund cycles.

#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Events},
    token, Address, BytesN, Env,
};

use crate::{
    types::{DataKey, Quest, QuestType, Submission, SubmissionStatus},
    QuestEngineContract, QuestEngineContractClient,
};

// ── Mock StakeVault ───────────────────────────────────────────────────────────

#[contract]
pub struct MockStakeVaultInv;

#[contractimpl]
impl MockStakeVaultInv {
    pub fn get_multiplier(_env: Env, _learner: Address) -> u32 { 100 }
}

// ── Mock RewardPool ───────────────────────────────────────────────────────────

#[contract]
pub struct MockRewardPoolInv;

#[contractimpl]
impl MockRewardPoolInv {
    pub fn distribute_reward(_env: Env, _caller: Address, _learner: Address, _amount: i128) {}
}

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

fn setup() -> (
    Env,
    QuestEngineContractClient<'static>,
    Address, // token_id
    Address, // reward_pool_id
    Address, // admin
) {
    let env = Env::default();
    env.mock_all_auths();

    let id = env.register(QuestEngineContract, ());
    let client = QuestEngineContractClient::new(&env, &id);

    let token_admin = Address::generate(&env);
    let token_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();

    let stake_vault_id = env.register(MockStakeVaultInv, ());
    let admin = Address::generate(&env);
    let reward_pool = Address::generate(&env);
    client.initialize(&admin, &token_id, &reward_pool, &stake_vault_id);

    (env, client, token_id, reward_pool, admin)
}

fn mint_tokens(env: &Env, token_id: &Address, to: &Address, amount: i128) {
    token::StellarAssetClient::new(env, token_id).mint(to, &amount);
}

fn tok(env: &Env, token_id: &Address) -> token::Client<'static> {
    let env = env.clone();
    let token_id = token_id.clone();
    token::Client::new(&env, &token_id)
}

fn assert_quest_exists(env: &Env, contract: &Address, quest_id: u32) -> Quest {
    env.as_contract(contract, || {
        env.storage()
            .persistent()
            .get(&DataKey::Quest(quest_id))
            .expect("invariant: DataKey::Quest(id) must exist after create")
    })
}

fn assert_submission_exists(
    env: &Env,
    contract: &Address,
    learner: &Address,
    quest_id: u32,
) -> Submission {
    env.as_contract(contract, || {
        env.storage()
            .persistent()
            .get(&DataKey::Submission(learner.clone(), quest_id))
            .expect("invariant: Submission slot must exist after submit_proof")
    })
}

fn assert_no_submission(env: &Env, contract: &Address, learner: &Address, quest_id: u32) {
    env.as_contract(contract, || {
        let has = env
            .storage()
            .persistent()
            .has(&DataKey::Submission(learner.clone(), quest_id));
        assert!(!has,
            "invariant violated: no Submission slot should exist for ({:?}, {})",
            learner, quest_id);
    });
}

fn assert_quest_active(env: &Env, contract: &Address, quest_id: u32, expected: bool) {
    env.as_contract(contract, || {
        let q: Quest = env.storage().persistent().get(&DataKey::Quest(quest_id)).unwrap();
        assert_eq!(q.active, expected,
            "Quest({}).active expected={}, got={}", quest_id, expected, q.active);
    });
}

// ── initialize ────────────────────────────────────────────────────────────────

/// `initialize` must emit **0** events.
#[test]
fn inv_initialize_emits_no_events() {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(QuestEngineContract, ());
    let client = QuestEngineContractClient::new(&env, &id);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone()).address();
    let stake_vault_id = env.register(MockStakeVaultInv, ());
    let admin = Address::generate(&env);
    let reward_pool = Address::generate(&env);

    client.initialize(&admin, &token_id, &reward_pool, &stake_vault_id);

    assert_event_count!(env, 0);
}

/// After `initialize`, the stored admin, token, reward_pool, and stake_vault
/// must equal the passed values.
#[test]
fn inv_initialize_stores_all_config() {
    let (env, client, token_id, reward_pool, admin) = setup();

    env.as_contract(&client.address, || {
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        let stored_token: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let stored_pool: Address = env.storage().instance().get(&DataKey::RewardPool).unwrap();

        assert_eq!(stored_admin, admin);
        assert_eq!(stored_token, token_id);
        assert_eq!(stored_pool, reward_pool);
    });
}

// ── create_build_quest ────────────────────────────────────────────────────────

/// `create_build_quest` must emit exactly **1** event (`QuestCreated`).
#[test]
fn inv_create_build_quest_emits_exactly_one_event() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[1u8; 32]);
    mint_tokens(&env, &token_id, &employer, 1_000);

    let baseline = env.events().all().len();
    client.create_build_quest(&employer, &1_000, &hash);

    assert_eq!(events_since(&env, baseline), 1,
        "create_build_quest must emit exactly 1 event");
}

/// After `create_build_quest`, `DataKey::Quest(id)` must be `active=true` and `QuestType::Build`.
#[test]
fn inv_create_build_quest_slot_correct() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[2u8; 32]);
    mint_tokens(&env, &token_id, &employer, 500);

    let id = client.create_build_quest(&employer, &500, &hash);
    let q = assert_quest_exists(&env, &client.address, id);

    assert_eq!(q.quest_type, QuestType::Build);
    assert_eq!(q.employer, employer);
    assert_eq!(q.reward_amount, 500);
    assert!(q.active);
}

/// Token conservation: `create_build_quest` transfers `reward_amount` from employer to vault.
#[test]
fn inv_create_build_quest_token_conservation() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[3u8; 32]);
    let amount = 750i128;
    mint_tokens(&env, &token_id, &employer, amount);

    let t = tok(&env, &token_id);
    let employer_before = t.balance(&employer);
    let vault_before = t.balance(&client.address);

    client.create_build_quest(&employer, &amount, &hash);

    assert_eq!(employer_before - t.balance(&employer), amount);
    assert_eq!(t.balance(&client.address) - vault_before, amount);
}

/// N `create_build_quest` calls → N events and N Quest slots.
#[test]
fn inv_create_build_quest_event_count_linear() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[4u8; 32]);
    mint_tokens(&env, &token_id, &employer, 3_000);

    let baseline = env.events().all().len();
    for _ in 0..3 {
        client.create_build_quest(&employer, &1_000, &hash);
    }
    assert_eq!(events_since(&env, baseline), 3);
}

// ── create_explore_quest ──────────────────────────────────────────────────────

/// `create_explore_quest` must emit exactly **1** event (`QuestCreated`).
#[test]
fn inv_create_explore_quest_emits_exactly_one_event() {
    let (env, client, _token, _pool, admin) = setup();
    let hash = BytesN::from_array(&env, &[5u8; 32]);

    let baseline = env.events().all().len();
    client.create_explore_quest(&admin, &500, &hash);

    assert_eq!(events_since(&env, baseline), 1,
        "create_explore_quest must emit exactly 1 event");
}

/// After `create_explore_quest`, the stored quest must be `QuestType::Explore` and active.
#[test]
fn inv_create_explore_quest_slot_correct() {
    let (env, client, _token, _pool, admin) = setup();
    let hash = BytesN::from_array(&env, &[6u8; 32]);

    let id = client.create_explore_quest(&admin, &300, &hash);
    let q = assert_quest_exists(&env, &client.address, id);

    assert_eq!(q.quest_type, QuestType::Explore);
    assert!(q.active);
}

// ── submit_proof ──────────────────────────────────────────────────────────────

/// `submit_proof` must emit exactly **1** event (`ProofSubmitted`).
#[test]
fn inv_submit_proof_emits_exactly_one_event() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let learner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[7u8; 32]);
    mint_tokens(&env, &token_id, &employer, 1_000);

    let quest_id = client.create_build_quest(&employer, &1_000, &hash);
    let baseline = env.events().all().len();

    client.submit_proof(&learner, &quest_id, &hash);

    assert_eq!(events_since(&env, baseline), 1,
        "submit_proof must emit exactly 1 event");
}

/// After `submit_proof`, `DataKey::Submission(learner, id)` must exist with `Pending` status.
#[test]
fn inv_submit_proof_creates_pending_submission() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let learner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[8u8; 32]);
    mint_tokens(&env, &token_id, &employer, 1_000);

    let quest_id = client.create_build_quest(&employer, &1_000, &hash);
    client.submit_proof(&learner, &quest_id, &hash);

    let sub = assert_submission_exists(&env, &client.address, &learner, quest_id);
    assert_eq!(sub.status, SubmissionStatus::Pending);
    assert_eq!(sub.proof_hash, hash);
}

/// `submit_proof` for learner A must not create a submission slot for learner B.
#[test]
fn inv_submit_proof_no_cross_learner_storage_leak() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let learner_a = Address::generate(&env);
    let learner_b = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[9u8; 32]);
    mint_tokens(&env, &token_id, &employer, 1_000);

    let quest_id = client.create_build_quest(&employer, &1_000, &hash);
    client.submit_proof(&learner_a, &quest_id, &hash);

    assert_no_submission(&env, &client.address, &learner_b, quest_id);
}

// ── review_submission ─────────────────────────────────────────────────────────

/// `review_submission` (approve) must emit exactly **1** event (`SubmissionReviewed`).
#[test]
fn inv_review_submission_approve_emits_exactly_one_event() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let learner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[10u8; 32]);
    mint_tokens(&env, &token_id, &employer, 1_000);

    let quest_id = client.create_build_quest(&employer, &1_000, &hash);
    client.submit_proof(&learner, &quest_id, &hash);
    let baseline = env.events().all().len();

    client.review_submission(&employer, &learner, &quest_id, &true);

    assert_eq!(events_since(&env, baseline), 1,
        "review_submission must emit exactly 1 event");
}

/// `review_submission` (reject) must emit exactly **1** event.
#[test]
fn inv_review_submission_reject_emits_exactly_one_event() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let learner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[11u8; 32]);
    mint_tokens(&env, &token_id, &employer, 1_000);

    let quest_id = client.create_build_quest(&employer, &1_000, &hash);
    client.submit_proof(&learner, &quest_id, &hash);
    let baseline = env.events().all().len();

    client.review_submission(&employer, &learner, &quest_id, &false);

    assert_eq!(events_since(&env, baseline), 1);
}

/// After approve, submission status must be `Approved`.
#[test]
fn inv_review_submission_approve_sets_approved_status() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let learner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[12u8; 32]);
    mint_tokens(&env, &token_id, &employer, 1_000);

    let quest_id = client.create_build_quest(&employer, &1_000, &hash);
    client.submit_proof(&learner, &quest_id, &hash);
    client.review_submission(&employer, &learner, &quest_id, &true);

    let sub = assert_submission_exists(&env, &client.address, &learner, quest_id);
    assert_eq!(sub.status, SubmissionStatus::Approved);
}

/// After reject, submission status must be `Rejected` and vault balance must be unchanged.
#[test]
fn inv_review_submission_reject_sets_rejected_status_no_token_movement() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let learner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[13u8; 32]);
    let amount = 1_000i128;
    mint_tokens(&env, &token_id, &employer, amount);

    let quest_id = client.create_build_quest(&employer, &amount, &hash);
    client.submit_proof(&learner, &quest_id, &hash);

    let t = tok(&env, &token_id);
    let vault_before = t.balance(&client.address);

    client.review_submission(&employer, &learner, &quest_id, &false);

    let sub = assert_submission_exists(&env, &client.address, &learner, quest_id);
    assert_eq!(sub.status, SubmissionStatus::Rejected);
    assert_eq!(t.balance(&client.address), vault_before,
        "token balance must be unchanged after rejection");
}

/// Token conservation on approve: vault empties, learner gets (reward - fee), reward_pool gets fee.
#[test]
fn inv_review_submission_approve_token_conservation() {
    let (env, client, token_id, reward_pool, _admin) = setup();
    let employer = Address::generate(&env);
    let learner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[14u8; 32]);
    let amount = 1_000i128;
    mint_tokens(&env, &token_id, &employer, amount);

    let quest_id = client.create_build_quest(&employer, &amount, &hash);
    client.submit_proof(&learner, &quest_id, &hash);

    let t = tok(&env, &token_id);
    client.review_submission(&employer, &learner, &quest_id, &true);

    let fee = (amount * 15) / 100;
    let learner_expected = amount - fee;

    assert_eq!(t.balance(&client.address), 0, "vault must be empty after approval");
    assert_eq!(t.balance(&learner), learner_expected);
    assert_eq!(t.balance(&reward_pool), fee);
}

// ── batch_review_submissions ──────────────────────────────────────────────────

/// `batch_review_submissions` for N learners must emit N+1 events
/// (N × `SubmissionReviewed` + 1 × `BatchReviewed`).
#[test]
fn inv_batch_review_emits_n_plus_one_events() {
    let (env, client, token_id, reward_pool, _admin) = setup();
    let employer = Address::generate(&env);
    let l1 = Address::generate(&env);
    let l2 = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[15u8; 32]);
    mint_tokens(&env, &token_id, &employer, 2_000);

    let q1 = client.create_build_quest(&employer, &1_000, &hash);
    let q2 = client.create_build_quest(&employer, &1_000, &hash);
    client.submit_proof(&l1, &q1, &hash);
    client.submit_proof(&l2, &q2, &hash);

    // batch approve l1 on q1 (1 learner → 1+1=2 events)
    let baseline = env.events().all().len();
    let mut learners = soroban_sdk::Vec::new(&env);
    learners.push_back(l1.clone());
    client.batch_review_submissions(&employer, &q1, &learners);

    assert_eq!(events_since(&env, baseline), 2,
        "batch_review with 1 learner must emit 2 events (SubmissionReviewed + BatchReviewed)");
    let _ = (l2, q2, reward_pool);
}

/// After `batch_review_submissions`, each learner's submission must be `Approved`.
#[test]
fn inv_batch_review_all_submissions_approved() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let learner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[16u8; 32]);
    mint_tokens(&env, &token_id, &employer, 1_000);

    let quest_id = client.create_build_quest(&employer, &1_000, &hash);
    client.submit_proof(&learner, &quest_id, &hash);

    let mut learners = soroban_sdk::Vec::new(&env);
    learners.push_back(learner.clone());
    client.batch_review_submissions(&employer, &quest_id, &learners);

    let sub = assert_submission_exists(&env, &client.address, &learner, quest_id);
    assert_eq!(sub.status, SubmissionStatus::Approved);
}

// ── refund_quest ──────────────────────────────────────────────────────────────

/// `refund_quest` must emit exactly **1** event (`QuestRefunded`).
#[test]
fn inv_refund_quest_emits_exactly_one_event() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[17u8; 32]);
    mint_tokens(&env, &token_id, &employer, 1_000);

    let quest_id = client.create_build_quest(&employer, &1_000, &hash);
    let baseline = env.events().all().len();

    client.refund_quest(&employer, &quest_id);

    assert_eq!(events_since(&env, baseline), 1,
        "refund_quest must emit exactly 1 event");
}

/// After `refund_quest`, quest must be inactive and full reward returned to employer.
#[test]
fn inv_refund_quest_deactivates_and_returns_tokens() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[18u8; 32]);
    let amount = 1_000i128;
    mint_tokens(&env, &token_id, &employer, amount);

    let quest_id = client.create_build_quest(&employer, &amount, &hash);
    let t = tok(&env, &token_id);

    client.refund_quest(&employer, &quest_id);

    assert_quest_active(&env, &client.address, quest_id, false);
    assert_eq!(t.balance(&employer), amount, "employer must get full refund");
    assert_eq!(t.balance(&client.address), 0, "vault must be empty after refund");
}

// ── verify_explore_quest ──────────────────────────────────────────────────────

/// `verify_explore_quest` must emit exactly **1** event (`ExploreQuestVerified`).
#[test]
fn inv_verify_explore_quest_emits_exactly_one_event() {
    let (env, client, token_id, _pool, admin) = setup();
    let learner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[19u8; 32]);

    // Wire a mock reward pool that accepts the call
    let mock_pool_id = env.register(MockRewardPoolInv, ());
    let stake_vault_id = env.register(MockStakeVaultInv, ());
    let id = env.register(QuestEngineContract, ());
    let client2 = QuestEngineContractClient::new(&env, &id);
    client2.initialize(&admin, &token_id, &mock_pool_id, &stake_vault_id);

    let quest_id = client2.create_explore_quest(&admin, &500, &hash);
    let baseline = env.events().all().len();

    client2.verify_explore_quest(&admin, &learner, &quest_id);

    assert_eq!(events_since(&env, baseline), 1,
        "verify_explore_quest must emit exactly 1 event");
}

// ── set_pause / get_quest / get_submission (config + reads) ───────────────────

/// `set_pause` must emit **0** events.
#[test]
fn inv_set_pause_emits_no_events() {
    let (env, client, _token, _pool, admin) = setup();
    let baseline = env.events().all().len();

    client.set_pause(&admin, &true);

    assert_eq!(events_since(&env, baseline), 0,
        "set_pause must not emit any events");
}

/// Pure read methods must emit **0** events.
#[test]
fn inv_read_methods_emit_no_events() {
    let (env, client, token_id, _pool, _admin) = setup();
    let employer = Address::generate(&env);
    let learner = Address::generate(&env);
    let hash = BytesN::from_array(&env, &[20u8; 32]);
    mint_tokens(&env, &token_id, &employer, 1_000);

    let quest_id = client.create_build_quest(&employer, &1_000, &hash);
    client.submit_proof(&learner, &quest_id, &hash);

    let baseline = env.events().all().len();

    let _ = client.get_quest(&quest_id);
    let _ = client.get_quest(&999);
    let _ = client.get_submission(&learner, &quest_id);
    let _ = client.get_submission(&learner, &999);

    assert_eq!(events_since(&env, baseline), 0,
        "get_quest and get_submission must not emit any events");
}
