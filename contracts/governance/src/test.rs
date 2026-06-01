use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

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
