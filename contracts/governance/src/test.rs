use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    Address, BytesN, Env,
};

use badge_nft::{BadgeNFT, BadgeNFTClient};

use crate::{types::DataKey, Governance, GovernanceClient, Proposal};

fn setup() -> (Env, GovernanceClient<'static>, BadgeNFTClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let governance_id = env.register(Governance, ());
    let badge_nft_id = env.register(BadgeNFT, ());

    let governance_client = GovernanceClient::new(&env, &governance_id);
    let badge_client = BadgeNFTClient::new(&env, &badge_nft_id);

    (env, governance_client, badge_client)
}

fn dummy_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[7u8; 32])
}

fn seed_proposal(
    env: &Env,
    governance_client: &GovernanceClient<'_>,
    proposal_id: u32,
    proposer: &Address,
) {
    env.as_contract(&governance_client.address, || {
        env.storage().persistent().set(
            &DataKey::Proposal(proposal_id),
            &Proposal {
                id: proposal_id,
                proposer: proposer.clone(),
                metadata_hash: dummy_hash(env),
                votes_for: 0,
                votes_against: 0,
                end_time: 1_000,
                executed: false,
            },
        );
    });
}

#[test]
fn test_cast_vote_uses_badge_count_as_weight() {
    let (env, governance_client, badge_client) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    governance_client.initialize(&badge_client.address);
    badge_client.initialize(&badge_admin);
    seed_proposal(&env, &governance_client, 1, &proposer);

    badge_client.mint_badge(&badge_admin, &voter, &101);
    badge_client.mint_badge(&badge_admin, &voter, &102);
    badge_client.mint_badge(&badge_admin, &voter, &103);

    governance_client.cast_vote(&voter, &1, &true);

    let proposal = governance_client.get_proposal(&1);
    assert_eq!(proposal.votes_for, 3);
    assert_eq!(proposal.votes_against, 0);

    env.as_contract(&governance_client.address, || {
        let recorded: bool = env
            .storage()
            .persistent()
            .get(&DataKey::UserVote(voter.clone(), 1))
            .expect("vote should be recorded");
        assert!(recorded);
    });
}

#[test]
#[should_panic(expected = "Already voted")]
fn test_cast_vote_prevents_double_voting() {
    let (env, governance_client, badge_client) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    governance_client.initialize(&badge_client.address);
    badge_client.initialize(&badge_admin);
    seed_proposal(&env, &governance_client, 1, &proposer);

    badge_client.mint_badge(&badge_admin, &voter, &999);

    governance_client.cast_vote(&voter, &1, &true);
    governance_client.cast_vote(&voter, &1, &false);
}

#[test]
fn test_execute_proposal_success() {
    let (env, governance_client, badge_client) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    governance_client.initialize(&badge_client.address);
    badge_client.initialize(&badge_admin);
    seed_proposal(&env, &governance_client, 1, &proposer);

    // Cast votes in favor
    badge_client.mint_badge(&badge_admin, &voter, &101);
    badge_client.mint_badge(&badge_admin, &voter, &102);
    governance_client.cast_vote(&voter, &1, &true);

    // Move time past end_time
    env.ledger().with_mut(|li| li.timestamp = 1_001);

    governance_client.execute_proposal(&1);

    let proposal = governance_client.get_proposal(&1);
    assert!(proposal.executed);
    assert_eq!(proposal.votes_for, 2);
    assert_eq!(proposal.votes_against, 0);
}

#[test]
#[should_panic(expected = "Voting still active")]
fn test_execute_proposal_voting_still_active() {
    let (env, governance_client, badge_client) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    governance_client.initialize(&badge_client.address);
    badge_client.initialize(&badge_admin);
    seed_proposal(&env, &governance_client, 1, &proposer);

    // Cast votes in favor
    badge_client.mint_badge(&badge_admin, &voter, &101);
    governance_client.cast_vote(&voter, &1, &true);

    // Time is still before end_time (1_000)
    env.ledger().with_mut(|li| li.timestamp = 999);

    governance_client.execute_proposal(&1);
}

#[test]
#[should_panic(expected = "Proposal rejected")]
fn test_execute_proposal_rejected() {
    let (env, governance_client, badge_client) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter_for = Address::generate(&env);
    let voter_against = Address::generate(&env);

    governance_client.initialize(&badge_client.address);
    badge_client.initialize(&badge_admin);
    seed_proposal(&env, &governance_client, 1, &proposer);

    // Cast votes: 1 for, 2 against
    badge_client.mint_badge(&badge_admin, &voter_for, &101);
    governance_client.cast_vote(&voter_for, &1, &true);

    badge_client.mint_badge(&badge_admin, &voter_against, &201);
    badge_client.mint_badge(&badge_admin, &voter_against, &202);
    governance_client.cast_vote(&voter_against, &1, &false);

    // Move time past end_time
    env.ledger().with_mut(|li| li.timestamp = 1_001);

    governance_client.execute_proposal(&1);
}

#[test]
#[should_panic(expected = "Proposal rejected")]
fn test_execute_proposal_tied_vote() {
    let (env, governance_client, badge_client) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter_for = Address::generate(&env);
    let voter_against = Address::generate(&env);

    governance_client.initialize(&badge_client.address);
    badge_client.initialize(&badge_admin);
    seed_proposal(&env, &governance_client, 1, &proposer);

    // Cast votes: 2 for, 2 against (tie)
    badge_client.mint_badge(&badge_admin, &voter_for, &101);
    badge_client.mint_badge(&badge_admin, &voter_for, &102);
    governance_client.cast_vote(&voter_for, &1, &true);

    badge_client.mint_badge(&badge_admin, &voter_against, &201);
    badge_client.mint_badge(&badge_admin, &voter_against, &202);
    governance_client.cast_vote(&voter_against, &1, &false);

    // Move time past end_time
    env.ledger().with_mut(|li| li.timestamp = 1_001);

    governance_client.execute_proposal(&1);
}

#[test]
#[should_panic(expected = "Already executed")]
fn test_execute_proposal_already_executed() {
    let (env, governance_client, badge_client) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    governance_client.initialize(&badge_client.address);
    badge_client.initialize(&badge_admin);
    seed_proposal(&env, &governance_client, 1, &proposer);

    // Cast votes in favor
    badge_client.mint_badge(&badge_admin, &voter, &101);
    governance_client.cast_vote(&voter, &1, &true);

    // Move time past end_time
    env.ledger().with_mut(|li| li.timestamp = 1_001);

    governance_client.execute_proposal(&1);
    governance_client.execute_proposal(&1); // Try to execute again
}

#[test]
fn test_execute_proposal_emits_event() {
    let (env, governance_client, badge_client) = setup();
    let badge_admin = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);

    governance_client.initialize(&badge_client.address);
    badge_client.initialize(&badge_admin);
    seed_proposal(&env, &governance_client, 1, &proposer);

    // Cast votes in favor
    badge_client.mint_badge(&badge_admin, &voter, &101);
    governance_client.cast_vote(&voter, &1, &true);

    // Move time past end_time
    env.ledger().with_mut(|li| li.timestamp = 1_001);

    governance_client.execute_proposal(&1);

    // Verify event was emitted
    assert_eq!(env.events().all().len(), 1);
}

#[test]
#[should_panic(expected = "Proposal not found")]
fn test_execute_proposal_nonexistent() {
    let (env, governance_client, badge_client) = setup();

    governance_client.initialize(&badge_client.address);

    // Move time past end_time
    env.ledger().with_mut(|li| li.timestamp = 1_001);

    governance_client.execute_proposal(&999);
}
