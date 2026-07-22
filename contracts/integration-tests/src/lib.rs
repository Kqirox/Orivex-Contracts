#![cfg(test)]

//! Multi-contract integration tests for the Orivex platform.
//!
//! These tests deploy all six contracts against the same Soroban test env
//! and verify the full learner journey end-to-end.

use soroban_sdk::{
    testutils::{Address as _, Events},
    Address, BytesN, Env, String,
};

use badge_nft::{BadgeNFT, BadgeNFTClient};
use course_registry::{CourseRegistry, CourseRegistryClient};
use governance::{Governance, GovernanceClient};
use quest_engine::{QuestEngineContract, QuestEngineContractClient};
use reward_pool::{RewardPool, RewardPoolClient};
use soroban_sdk::token;
use stake_vault::{StakeVault, StakeVaultClient};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn dummy_hash(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[1u8; 32])
}

/// Complete deployment and wiring of all contracts.
/// Returns (admin, token_admin, token_address, clients...).
fn setup_full_platform() -> (
    Env,
    Address, // admin
    Address, // token_admin
    Address, // token_address
    CourseRegistryClient<'static>,
    BadgeNFTClient<'static>,
    RewardPoolClient<'static>,
    QuestEngineContractClient<'static>,
    StakeVaultClient<'static>,
    GovernanceClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);

    // Deploy SAC token
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    // Deploy RewardPool
    let reward_pool_id = env.register(RewardPool, ());
    let reward_pool_client = RewardPoolClient::new(&env, &reward_pool_id);

    // Deploy BadgeNFT
    let badge_nft_id = env.register(BadgeNFT, ());
    let badge_nft_client = BadgeNFTClient::new(&env, &badge_nft_id);

    // Deploy StakeVault
    let stake_vault_id = env.register(StakeVault, ());
    let stake_vault_client = StakeVaultClient::new(&env, &stake_vault_id);

    // Deploy CourseRegistry
    let course_registry_id = env.register(CourseRegistry, ());
    let course_registry_client = CourseRegistryClient::new(&env, &course_registry_id);

    // Deploy QuestEngine (needs reward_pool and stake_vault addresses)
    let quest_engine_id = env.register(QuestEngineContract, ());
    let quest_engine_client = QuestEngineContractClient::new(&env, &quest_engine_id);

    // Deploy Governance (needs badge_nft address)
    let governance_id = env.register(Governance, ());
    let governance_client = GovernanceClient::new(&env, &governance_id);

    // ── Initialize contracts ─────────────────────────────────────────────

    // RewardPool: admin = platform admin, token = SAC token
    reward_pool_client.initialize(&admin, &token_address);

    // BadgeNFT: admin = CourseRegistry address (only it can mint badges)
    badge_nft_client.initialize(&course_registry_client.address);

    // StakeVault: admin = platform admin, token = SAC token
    stake_vault_client.initialize(&admin, &token_address);

    // QuestEngine: admin, token, reward_pool, stake_vault
    quest_engine_client.initialize(
        &admin,
        &token_address,
        &reward_pool_client.address,
        &stake_vault_client.address,
    );

    // Governance: admin, badge_contract_address
    governance_client.initialize(&admin, &badge_nft_client.address);

    // CourseRegistry: admin only
    course_registry_client.initialize(&admin);

    // ── Wire contracts ───────────────────────────────────────────────────

    // Whitelist CourseRegistry in RewardPool so it can distribute rewards
    let token_sac = token::StellarAssetClient::new(&env, &token_address);
    token_sac.mint(&reward_pool_client.address, &1_000_000_000); // 100 USDC
    reward_pool_client.add_approved_spender(&admin, &course_registry_client.address);

    // Wire CourseRegistry -> RewardPool
    course_registry_client.set_reward_pool_address(&admin, &reward_pool_client.address);

    // Wire CourseRegistry -> BadgeNFT
    course_registry_client.set_badge_nft_address(&admin, &badge_nft_client.address);

    // Whitelist QuestEngine in RewardPool for explore quest payouts
    reward_pool_client.add_approved_spender(&admin, &quest_engine_client.address);

    (
        env,
        admin,
        token_admin,
        token_address,
        course_registry_client,
        badge_nft_client,
        reward_pool_client,
        quest_engine_client,
        stake_vault_client,
        governance_client,
    )
}

// ── Test 1: Full Learner Journey E2E ─────────────────────────────────────────

#[test]
fn test_full_learner_journey_e2e() {
    let (
        env,
        admin,
        _token_admin,
        _token_address,
        course_registry,
        badge_nft,
        reward_pool,
        _quest_engine,
        _stake_vault,
        _governance,
    ) = setup_full_platform();

    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    // 1. Admin creates a 2-module course
    let course_id = course_registry.create_course(&admin, &instructor, &2, &dummy_hash(&env));
    assert_eq!(course_id, 1);

    // 2. Learner enrolls
    course_registry.enroll(&learner, &course_id);

    // 3. Learner completes module 1 (not final)
    course_registry.complete_module(&admin, &learner, &course_id);
    assert_eq!(course_registry.get_progress(&learner, &course_id), 1);
    assert!(!badge_nft.has_badge(&learner, &course_id));

    // 4. Learner completes module 2 (final) — badge minted, reward distributed
    course_registry.complete_module(&admin, &learner, &course_id);
    assert_eq!(course_registry.get_progress(&learner, &course_id), 2);

    // Badge should be minted
    assert!(badge_nft.has_badge(&learner, &course_id));
    assert_eq!(badge_nft.get_badge_count(&learner), 1);

    // Reward should be distributed (10 USDC)
    let token_sac = token::StellarAssetClient::new(&env, &_token_address);
    assert_eq!(token_sac.balance(&learner), 10_000_000);

    // 5. Verify course is finished
    assert!(course_registry.is_course_finished(&learner, &course_id));
}

// ── Test 2: Governance After Course Completion ────────────────────────────────

#[test]
fn test_governance_after_course_completion() {
    let (
        env,
        admin,
        _token_admin,
        _token_address,
        course_registry,
        badge_nft,
        _reward_pool,
        _quest_engine,
        _stake_vault,
        governance,
    ) = setup_full_platform();

    let instructor = Address::generate(&env);
    let learner = Address::generate(&env);

    // 1. Complete a course to earn a badge
    let course_id = course_registry.create_course(&admin, &instructor, &1, &dummy_hash(&env));
    course_registry.enroll(&learner, &course_id);
    course_registry.complete_module(&admin, &learner, &course_id);

    // Verify learner has 1 badge
    assert_eq!(badge_nft.get_badge_count(&learner), 1);

    // 2. Create a governance proposal
    let proposal_id = governance.create_proposal(&learner, &dummy_hash(&env));
    assert_eq!(proposal_id, 1);

    // 3. Learner casts a vote weighted by badge count (1 badge = 1 vote)
    governance.cast_vote(&learner, &proposal_id, &true);

    // 4. Check proposal state
    let proposal = governance.get_proposal(&proposal_id);
    assert_eq!(proposal.votes_for, 1);
    assert_eq!(proposal.votes_against, 0);
}

// ── Test 3: Explore Quest Payout Drains Reward Pool ───────────────────────────

#[test]
fn test_explore_quest_payout_drains_reward_pool_correctly() {
    let (
        env,
        admin,
        _token_admin,
        _token_address,
        _course_registry,
        _badge_nft,
        reward_pool,
        quest_engine,
        _stake_vault,
        _governance,
    ) = setup_full_platform();

    let employer = Address::generate(&env);
    let learner = Address::generate(&env);

    // Fund the quest with 5 USDC
    let quest_reward: i128 = 5_000_000;

    // 1. Employer creates an Explore quest
    let quest_id = quest_engine.create_quest(
        &employer,
        &quest_reward,
        &quest_engine::QuestType::Explore,
        &dummy_hash(&env),
    );

    // 2. Learner submits proof
    quest_engine.submit_proof(&learner, &quest_id, &dummy_hash(&env));

    // 3. Admin verifies the explore quest (triggers payout from RewardPool)
    quest_engine.verify_explore_quest(&admin, &learner, &quest_id);

    // 4. Verify learner received the reward
    let token_sac = token::StellarAssetClient::new(&env, &_token_address);
    assert_eq!(token_sac.balance(&learner), quest_reward);

    // 5. Verify reward pool balance decreased
    assert_eq!(
        token_sac.balance(&reward_pool.address),
        1_000_000_000 - quest_reward
    );
}

// ── Test 4: Missing Wiring Causes Clear Failure ──────────────────────────────

#[test]
#[should_panic(expected = "Caller is not an authorized spender")]
fn test_missing_wiring_causes_failure() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let learner = Address::generate(&env);
    let instructor = Address::generate(&env);

    // Deploy SAC token
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    // Deploy RewardPool
    let reward_pool_id = env.register(RewardPool, ());
    let reward_pool_client = RewardPoolClient::new(&env, &reward_pool_id);

    // Deploy BadgeNFT
    let badge_nft_id = env.register(BadgeNFT, ());
    let badge_nft_client = BadgeNFTClient::new(&env, &badge_nft_id);

    // Deploy CourseRegistry
    let course_registry_id = env.register(CourseRegistry, ());
    let course_registry_client = CourseRegistryClient::new(&env, &course_registry_id);

    // Initialize contracts
    reward_pool_client.initialize(&admin, &token_address);
    badge_nft_client.initialize(&course_registry_client.address);
    course_registry_client.initialize(&admin);

    // Fund reward pool but do NOT whitelist CourseRegistry
    let token_sac = token::StellarAssetClient::new(&env, &token_address);
    token_sac.mint(&reward_pool_client.address, &1_000_000_000);

    // Wire CourseRegistry -> RewardPool (but skip add_approved_spender!)
    course_registry_client.set_reward_pool_address(&admin, &reward_pool_client.address);
    course_registry_client.set_badge_nft_address(&admin, &badge_nft_client.address);

    // Create and complete a course — should panic because CourseRegistry is not whitelisted
    let course_id =
        course_registry_client.create_course(&admin, &instructor, &1, &dummy_hash(&env));
    course_registry_client.enroll(&learner, &course_id);
    course_registry_client.complete_module(&admin, &learner, &course_id);
}

// ── Test 5: Multiple Learners Complete Courses Independently ──────────────────

#[test]
fn test_multiple_learners_independent_rewards() {
    let (
        env,
        admin,
        _token_admin,
        _token_address,
        course_registry,
        badge_nft,
        _reward_pool,
        _quest_engine,
        _stake_vault,
        _governance,
    ) = setup_full_platform();

    let instructor = Address::generate(&env);
    let learner_a = Address::generate(&env);
    let learner_b = Address::generate(&env);

    // Create a 1-module course
    let course_id = course_registry.create_course(&admin, &instructor, &1, &dummy_hash(&env));

    // Both learners complete the course
    course_registry.enroll(&learner_a, &course_id);
    course_registry.complete_module(&admin, &learner_a, &course_id);

    course_registry.enroll(&learner_b, &course_id);
    course_registry.complete_module(&admin, &learner_b, &course_id);

    // Both should have badges
    assert!(badge_nft.has_badge(&learner_a, &course_id));
    assert!(badge_nft.has_badge(&learner_b, &course_id));

    // Both should have received rewards
    let token_sac = token::StellarAssetClient::new(&env, &_token_address);
    assert_eq!(token_sac.balance(&learner_a), 10_000_000);
    assert_eq!(token_sac.balance(&learner_b), 10_000_000);

    // Reward pool should have decreased by 20 USDC
    assert_eq!(
        token_sac.balance(&_reward_pool.address),
        1_000_000_000 - 2 * 10_000_000
    );
}
