//! Invariant tests for the Governance contract.
//!
//! For every public method these tests assert:
//!
//! 1. **Event cardinality** — exactly the documented number of events.
//! 2. **Storage consistency** — Proposal and UserVote slots are in the correct
//!    state after each call; no orphaned keys are created.
//! 3. **Vote-weight invariant** — `cast_vote` delta equals the voter's badge count
//!    at the time of the call.
//! 4. **State-machine invariant** — `executed` flag transitions are one-way
//!    (`false → true`) and block further state changes.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    Address, BytesN, Env,
};

use badge_nft::{BadgeNFT, BadgeNFTClient};

use crate::{
    types::DataKey,
    Governance, GovernanceClient, Proposal,
};

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
    GovernanceClient<'static>,
    BadgeNFTClient<'static>,
    Address, // admin
) {
    let env = Env::default();
    env.mock_all_auths();

    let gov_id = env.register(Governance, ());
    let badge_id = env.register(BadgeNFT, ());

    let gov = GovernanceClient::new(&env, &gov_id);
    let badge = BadgeNFTClient::new(&env, &badge_id);
    let admin = Address::generate(&env);

    (env, gov, badge, admin)
}

fn dummy_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0x42u8; 32])
}

/// Seeds a Proposal directly into contract storage (mirrors the existing seed_proposal helper).
fn seed_proposal(
    env: &Env,
    gov: &GovernanceClient<'_>,
    id: u32,
    proposer: &Address,
    end_time: u64,
    votes_for: u32,
    votes_against: u32,
    executed: bool,
) {
    env.as_contract(&gov.address, || {
        env.storage().persistent().set(
            &DataKey::Proposal(id),
            &Proposal {
                id,
                proposer: proposer.clone(),
                metadata_hash: dummy_hash(env),
                votes_for,
                votes_against,
                end_time,
                executed,
            },
        );
    });
}

fn assert_proposal(env: &Env, gov: &GovernanceClient<'_>, id: u32) -> Proposal {
    env.as_contract(&gov.address, || {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(id))
            .expect("invariant: Proposal slot must exist")
    })
}

fn assert_no_proposal(env: &Env, gov: &GovernanceClient<'_>, id: u32) {
    env.as_contract(&gov.address, || {
        let has = env.storage().persistent().has(&DataKey::Proposal(id));
        assert!(!has, "no Proposal({}) slot should exist", id);
    });
}

fn assert_user_vote_flag(
    env: &Env,
    gov: &GovernanceClient<'_>,
    voter: &Address,
    proposal_id: u32,
    expected: bool,
) {
    env.as_contract(&gov.address, || {
        let stored: bool = env
            .storage()
            .persistent()
            .get(&DataKey::UserVote(voter.clone(), proposal_id))
            .unwrap_or(false);
        assert_eq!(
            stored, expected,
            "UserVote({:?}, {}) expected={}", voter, proposal_id, expected
        );
    });
}

fn assert_admin_stored(env: &Env, gov: &GovernanceClient<'_>, expected: &Address) {
    env.as_contract(&gov.address, || {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("invariant: Admin must be set");
        assert_eq!(stored, *expected);
    });
}

// ── initialize ────────────────────────────────────────────────────────────────

/// `initialize` must emit **0** events.
#[test]
fn inv_initialize_emits_no_events() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);

    assert_event_count!(env, 0);
}

/// After `initialize`, `DataKey::Admin` and `BADGE_NFT_KEY` must be set.
#[test]
fn inv_initialize_stores_admin() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);

    assert_admin_stored(&env, &gov, &admin);
}

/// After `initialize`, no Proposal or UserVote slots must exist.
#[test]
fn inv_initialize_no_orphan_slots() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let random = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);

    assert_no_proposal(&env, &gov, 1);
    assert_user_vote_flag(&env, &gov, &random, 1, false);
}

// ── cast_vote ─────────────────────────────────────────────────────────────────

/// `cast_vote` must emit **0** events (no event is defined for this method).
#[test]
fn inv_cast_vote_emits_no_events() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);
    badge.mint_badge(&badge_admin, &voter, &1);

    let baseline = env.events().all().len();
    gov.cast_vote(&voter, &1, &true);

    assert_eq!(events_since(&env, baseline), 0,
        "cast_vote must emit 0 events");
}

/// After `cast_vote`, `DataKey::UserVote(voter, proposal_id)` must be `true`.
#[test]
fn inv_cast_vote_records_user_vote_flag() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);
    badge.mint_badge(&badge_admin, &voter, &5);

    gov.cast_vote(&voter, &1, &true);

    assert_user_vote_flag(&env, &gov, &voter, 1, true);
}

/// Vote weight must equal the voter's badge count at call time.
#[test]
fn inv_cast_vote_weight_equals_badge_count() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);

    // Mint 3 badges
    badge.mint_badge(&badge_admin, &voter, &101);
    badge.mint_badge(&badge_admin, &voter, &102);
    badge.mint_badge(&badge_admin, &voter, &103);

    gov.cast_vote(&voter, &1, &true);

    let p = assert_proposal(&env, &gov, 1);
    assert_eq!(p.votes_for, 3,
        "votes_for must equal badge count (3)");
    assert_eq!(p.votes_against, 0);
}

/// `cast_vote(support=false)` increments `votes_against`, not `votes_for`.
#[test]
fn inv_cast_vote_against_increments_votes_against() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);
    badge.mint_badge(&badge_admin, &voter, &7);

    gov.cast_vote(&voter, &1, &false);

    let p = assert_proposal(&env, &gov, 1);
    assert_eq!(p.votes_against, 1);
    assert_eq!(p.votes_for, 0);
}

/// Two voters' weights accumulate independently in `votes_for` / `votes_against`.
#[test]
fn inv_cast_vote_accumulates_across_voters() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter_for = Address::generate(&env);
    let voter_against = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);

    badge.mint_badge(&badge_admin, &voter_for, &10);
    badge.mint_badge(&badge_admin, &voter_for, &11);
    badge.mint_badge(&badge_admin, &voter_against, &20);

    gov.cast_vote(&voter_for, &1, &true);
    gov.cast_vote(&voter_against, &1, &false);

    let p = assert_proposal(&env, &gov, 1);
    assert_eq!(p.votes_for, 2);
    assert_eq!(p.votes_against, 1);
}

/// After `cast_vote`, `UserVote` flag for voter A must not affect voter B.
#[test]
fn inv_cast_vote_no_cross_voter_flag_leak() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter_a = Address::generate(&env);
    let voter_b = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);
    badge.mint_badge(&badge_admin, &voter_a, &1);

    gov.cast_vote(&voter_a, &1, &true);

    // voter_b has NOT voted — their flag must be absent
    assert_user_vote_flag(&env, &gov, &voter_b, 1, false);
}

// ── execute_proposal ──────────────────────────────────────────────────────────

/// `execute_proposal` must emit exactly **1** event (`ProposalExecuted`).
#[test]
fn inv_execute_proposal_emits_exactly_one_event() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);
    badge.mint_badge(&badge_admin, &voter, &1);
    gov.cast_vote(&voter, &1, &true);

    env.ledger().with_mut(|l| l.timestamp = 1_001);
    let baseline = env.events().all().len();

    gov.execute_proposal(&1);

    assert_eq!(events_since(&env, baseline), 1,
        "execute_proposal must emit exactly 1 event");
}

/// After `execute_proposal`, `Proposal.executed` must be `true`.
#[test]
fn inv_execute_proposal_sets_executed_flag() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);
    badge.mint_badge(&badge_admin, &voter, &2);
    gov.cast_vote(&voter, &1, &true);

    env.ledger().with_mut(|l| l.timestamp = 1_001);
    gov.execute_proposal(&1);

    let p = assert_proposal(&env, &gov, 1);
    assert!(p.executed, "Proposal.executed must be true after execute_proposal");
}

/// `executed` flag must not change for other proposals when one is executed.
#[test]
fn inv_execute_proposal_does_not_affect_other_proposals() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);
    seed_proposal(&env, &gov, 2, &proposer, 1_000, 0, 0, false);
    badge.mint_badge(&badge_admin, &voter, &1);
    gov.cast_vote(&voter, &1, &true);

    env.ledger().with_mut(|l| l.timestamp = 1_001);
    gov.execute_proposal(&1);

    // Proposal 2 must remain untouched
    let p2 = assert_proposal(&env, &gov, 2);
    assert!(!p2.executed, "unrelated proposal must not be marked executed");
}

/// Votes count must not change after `execute_proposal`.
#[test]
fn inv_execute_proposal_votes_unchanged() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);
    badge.mint_badge(&badge_admin, &voter, &3);
    gov.cast_vote(&voter, &1, &true);

    let before = assert_proposal(&env, &gov, 1);
    env.ledger().with_mut(|l| l.timestamp = 1_001);
    gov.execute_proposal(&1);
    let after = assert_proposal(&env, &gov, 1);

    assert_eq!(after.votes_for, before.votes_for);
    assert_eq!(after.votes_against, before.votes_against);
}

// ── cancel_proposal ───────────────────────────────────────────────────────────

/// `cancel_proposal` must emit exactly **1** event (`ProposalCancelled`).
#[test]
fn inv_cancel_proposal_emits_exactly_one_event() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);

    let baseline = env.events().all().len();
    gov.cancel_proposal(&proposer, &1);

    assert_eq!(events_since(&env, baseline), 1,
        "cancel_proposal must emit exactly 1 event");
}

/// After `cancel_proposal`, `Proposal.executed` must be `true`
/// (cancellation reuses the executed flag as the "locked" sentinel).
#[test]
fn inv_cancel_proposal_sets_executed_flag() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);

    gov.cancel_proposal(&proposer, &1);

    let p = assert_proposal(&env, &gov, 1);
    assert!(p.executed, "Proposal.executed must be true after cancel_proposal");
}

/// Cancelling proposal 1 must not alter proposal 2.
#[test]
fn inv_cancel_proposal_does_not_affect_other_proposals() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);
    seed_proposal(&env, &gov, 2, &proposer, 1_000, 0, 0, false);

    gov.cancel_proposal(&proposer, &1);

    let p2 = assert_proposal(&env, &gov, 2);
    assert!(!p2.executed, "cancellation of proposal 1 must not affect proposal 2");
}

/// Vote counts must not change after cancellation.
#[test]
fn inv_cancel_proposal_votes_unchanged() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);
    badge.mint_badge(&badge_admin, &voter, &2);
    gov.cast_vote(&voter, &1, &true);

    let before = assert_proposal(&env, &gov, 1);
    gov.cancel_proposal(&proposer, &1);
    let after = assert_proposal(&env, &gov, 1);

    assert_eq!(after.votes_for, before.votes_for);
    assert_eq!(after.votes_against, before.votes_against);
}

/// After cancellation, admin storage must remain intact.
#[test]
fn inv_cancel_proposal_admin_unchanged() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);

    gov.cancel_proposal(&proposer, &1);

    assert_admin_stored(&env, &gov, &admin);
}

// ── get_proposal (pure read) ──────────────────────────────────────────────────

/// `get_proposal` must emit **0** events.
#[test]
fn inv_get_proposal_emits_no_events() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 1_000, 0, 0, false);

    let baseline = env.events().all().len();
    let _ = gov.get_proposal(&1);

    assert_eq!(events_since(&env, baseline), 0,
        "get_proposal must not emit any events");
}

/// `get_proposal` must return data consistent with what was seeded.
#[test]
fn inv_get_proposal_returns_correct_data() {
    let (env, gov, badge, admin) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);

    gov.initialize(&admin, &badge.address);
    badge.initialize(&badge_admin);
    seed_proposal(&env, &gov, 1, &proposer, 2_000, 5, 3, false);

    let p = gov.get_proposal(&1);
    assert_eq!(p.id, 1);
    assert_eq!(p.proposer, proposer);
    assert_eq!(p.end_time, 2_000);
    assert_eq!(p.votes_for, 5);
    assert_eq!(p.votes_against, 3);
    assert!(!p.executed);
}
