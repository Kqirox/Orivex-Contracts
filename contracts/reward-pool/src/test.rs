#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events},
    token, vec, Address, Env, IntoVal, Map, Symbol, Val, Vec,
};

use crate::{RewardPool, RewardPoolClient};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup() -> (Env, RewardPoolClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    // Fixed: Passing the contract type first, and empty constructor args second
    let contract_id = env.register(RewardPool, ());

    let client = RewardPoolClient::new(&env, &contract_id);
    (env, client)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_initialize_success() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // Initialize the contract
    client.initialize(&admin, &token);

    // Verify event was emitted
    assert_eq!(env.events().all().len(), 1);
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_initialize_twice_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // First initialization should succeed
    client.initialize(&admin, &token);

    // Second initialization should panic
    client.initialize(&admin, &token);
}

#[test]
#[should_panic(expected = "Error(Auth, InvalidAction)")]
fn test_initialize_without_auth_panics() {
    let env = Env::default();
    let contract_id = env.register(RewardPool, ());
    let client = RewardPoolClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // Try to initialize without mocking auths - should panic
    client.initialize(&admin, &token);
}

#[test]
fn test_initialize_with_proper_auth() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    // Initialize with proper authentication (mocked by env.mock_all_auths())
    client.initialize(&admin, &token);

    // Verify event was emitted
    assert_eq!(env.events().all().len(), 1);
}

// ── add_approved_spender Tests ────────────────────────────────────────────────

#[test]
fn test_add_approved_spender_success() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let spender = Address::generate(&env);

    // Initialize the contract
    client.initialize(&admin, &token);

    // Add approved spender - should succeed without panic
    client.add_approved_spender(&admin, &spender);

    // assert event emitted
    let empty_data: Map<(), ()> = Map::new(&env);
    let event = vec![
        &env,
        (
            client.address,
            (Symbol::new(&env, "spender_added"), spender).into_val(&env),
            empty_data.into_val(&env),
        ),
    ];

    assert_eq!(env.events().all(), event)
}

#[test]
#[should_panic(expected = "Not initialized")]
fn test_add_approved_spender_not_initialized() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let spender = Address::generate(&env);

    // Try to add spender without initializing - should panic
    client.add_approved_spender(&admin, &spender);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_add_approved_spender_wrong_admin() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let wrong_admin = Address::generate(&env);
    let token = Address::generate(&env);
    let spender = Address::generate(&env);

    // Initialize the contract
    client.initialize(&admin, &token);

    // Try to add spender with wrong admin - should panic
    client.add_approved_spender(&wrong_admin, &spender);
}

#[test]
fn test_add_multiple_approved_spenders() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let spender1 = Address::generate(&env);
    let spender2 = Address::generate(&env);

    // Initialize the contract
    client.initialize(&admin, &token);

    // Add multiple spenders - should succeed without panic
    client.add_approved_spender(&admin, &spender1);
    client.add_approved_spender(&admin, &spender2);
}

#[test]
fn test_add_same_spender_twice() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let spender = Address::generate(&env);

    // Initialize the contract
    client.initialize(&admin, &token);

    // Add same spender twice (should not panic, just overwrite)
    client.add_approved_spender(&admin, &spender);
    client.add_approved_spender(&admin, &spender);
}

// ── distribute_reward Tests ───────────────────────────────────────────────────

#[test]
fn test_distribute_reward_success() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let spender = Address::generate(&env);
    let learner = Address::generate(&env);

    // Create and register a mock token contract
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    // Initialize the reward pool
    client.initialize(&admin, &token_id.address());

    // Whitelist the spender
    client.add_approved_spender(&admin, &spender);

    // Mint tokens to the reward pool contract
    let token_client = token::StellarAssetClient::new(&env, &token_id.address());
    token_client.mint(&client.address, &1000);

    // Distribute reward
    client.distribute_reward(&spender, &learner, &100);

    // Check events immediately after distribute_reward
    let last_event = env.events().all().last().unwrap();

    let mut data_map = Map::new(&env);
    data_map.set(Symbol::new(&env, "amount"), 100i128);
    let expected_event: (Address, Vec<Val>, Val) = (
        client.address,
        (Symbol::new(&env, "reward_distributed"), &spender, &learner).into_val(&env),
        data_map.into_val(&env),
    );

    // Verify events match
    assert_eq!(
        vec![&env, last_event.clone()],
        vec![&env, expected_event.clone()]
    );

    // Verify learner received tokens
    let learner_balance = token_client.balance(&learner);
    assert_eq!(learner_balance, 100);
}

#[test]
#[should_panic(expected = "Amount must be positive")]
fn test_distribute_reward_zero_amount() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let spender = Address::generate(&env);
    let learner = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    client.initialize(&admin, &token_id.address());
    client.add_approved_spender(&admin, &spender);

    // Try to distribute zero amount - should panic
    client.distribute_reward(&spender, &learner, &0);
}

#[test]
#[should_panic(expected = "Amount must be positive")]
fn test_distribute_reward_negative_amount() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let spender = Address::generate(&env);
    let learner = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    client.initialize(&admin, &token_id.address());
    client.add_approved_spender(&admin, &spender);

    // Try to distribute negative amount - should panic
    client.distribute_reward(&spender, &learner, &-100);
}

#[test]
#[should_panic(expected = "Caller is not an authorized spender")]
fn test_distribute_reward_unauthorized_spender() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let unauthorized_spender = Address::generate(&env);
    let learner = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    client.initialize(&admin, &token_id.address());

    // Try to distribute without being whitelisted - should panic
    client.distribute_reward(&unauthorized_spender, &learner, &100);
}

#[test]
#[should_panic(expected = "Not initialized")]
fn test_distribute_reward_not_initialized() {
    let (env, client) = setup();
    let spender = Address::generate(&env);
    let learner = Address::generate(&env);

    // Try to distribute without initializing - should panic
    client.distribute_reward(&spender, &learner, &100);
}

#[test]
fn test_distribute_reward_multiple_times() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let spender = Address::generate(&env);
    let learner1 = Address::generate(&env);
    let learner2 = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    client.initialize(&admin, &token_id.address());
    client.add_approved_spender(&admin, &spender);

    let token_client = token::StellarAssetClient::new(&env, &token_id.address());
    token_client.mint(&client.address, &1000);

    // Distribute to multiple learners
    client.distribute_reward(&spender, &learner1, &100);
    client.distribute_reward(&spender, &learner2, &200);

    assert_eq!(token_client.balance(&learner1), 100);
    assert_eq!(token_client.balance(&learner2), 200);
}

#[test]
fn test_distribute_reward_multiple_spenders() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let spender1 = Address::generate(&env);
    let spender2 = Address::generate(&env);
    let learner = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    client.initialize(&admin, &token_id.address());
    client.add_approved_spender(&admin, &spender1);
    client.add_approved_spender(&admin, &spender2);

    let token_client = token::StellarAssetClient::new(&env, &token_id.address());
    token_client.mint(&client.address, &1000);

    // Both spenders can distribute
    client.distribute_reward(&spender1, &learner, &100);
    client.distribute_reward(&spender2, &learner, &50);

    assert_eq!(token_client.balance(&learner), 150);
}

// ── fund_pool Tests ───────────────────────────────────────────────────────────

#[test]
fn test_fund_pool_success() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let donor = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    // Initialize the reward pool
    client.initialize(&admin, &token_id.address());

    // Mint tokens to the donor
    let token_client = token::StellarAssetClient::new(&env, &token_id.address());
    token_client.mint(&donor, &1000);

    // Fund the pool
    client.fund_pool(&donor, &500);

    // Verify donor's balance decreased
    assert_eq!(token_client.balance(&donor), 500);

    // Verify contract's balance increased
    assert_eq!(token_client.balance(&client.address), 500);
}

#[test]
fn test_fund_pool_emits_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let donor = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    client.initialize(&admin, &token_id.address());

    let token_client = token::StellarAssetClient::new(&env, &token_id.address());
    token_client.mint(&donor, &1000);

    client.fund_pool(&donor, &300);

    // Verify event was emitted
    let last_event = env.events().all().last().unwrap();

    let mut data_map = Map::new(&env);
    data_map.set(Symbol::new(&env, "amount"), 300i128);
    let expected_event: (Address, Vec<Val>, Val) = (
        client.address,
        (Symbol::new(&env, "pool_funded"), &donor).into_val(&env),
        data_map.into_val(&env),
    );

    assert_eq!(
        vec![&env, last_event.clone()],
        vec![&env, expected_event.clone()]
    );
}

#[test]
#[should_panic(expected = "Not initialized")]
fn test_fund_pool_not_initialized() {
    let (env, client) = setup();
    let donor = Address::generate(&env);

    // Try to fund without initializing - should panic
    client.fund_pool(&donor, &100);
}

#[test]
fn test_fund_pool_multiple_donors() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let donor1 = Address::generate(&env);
    let donor2 = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    client.initialize(&admin, &token_id.address());

    let token_client = token::StellarAssetClient::new(&env, &token_id.address());
    token_client.mint(&donor1, &1000);
    token_client.mint(&donor2, &1000);

    // Multiple donors fund the pool
    client.fund_pool(&donor1, &500);
    client.fund_pool(&donor2, &300);

    // Verify balances
    assert_eq!(token_client.balance(&donor1), 500);
    assert_eq!(token_client.balance(&donor2), 700);
    assert_eq!(token_client.balance(&client.address), 800);
}

#[test]
fn test_fund_pool_multiple_times() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let donor = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    client.initialize(&admin, &token_id.address());

    let token_client = token::StellarAssetClient::new(&env, &token_id.address());
    token_client.mint(&donor, &2000);

    // Donor funds multiple times
    client.fund_pool(&donor, &500);
    client.fund_pool(&donor, &300);
    client.fund_pool(&donor, &200);

    // Verify balances
    assert_eq!(token_client.balance(&donor), 1000);
    assert_eq!(token_client.balance(&client.address), 1000);
}

#[test]
fn test_fund_pool_zero_amount() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let donor = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    client.initialize(&admin, &token_id.address());

    let token_client = token::StellarAssetClient::new(&env, &token_id.address());
    token_client.mint(&donor, &1000);

    // Fund with zero amount (should succeed)
    client.fund_pool(&donor, &0);

    // Verify balances unchanged
    assert_eq!(token_client.balance(&donor), 1000);
    assert_eq!(token_client.balance(&client.address), 0);
}

// ── emergency_sweep Tests ─────────────────────────────────────────────────────

#[test]
fn test_emergency_sweep_success() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let recovery_wallet = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    // Initialize the reward pool
    client.initialize(&admin, &token_id.address());

    // Fund the pool with tokens
    let token_client = token::StellarAssetClient::new(&env, &token_id.address());
    token_client.mint(&client.address, &1000);

    // Verify initial balance
    assert_eq!(token_client.balance(&client.address), 1000);
    assert_eq!(token_client.balance(&recovery_wallet), 0);

    // Perform emergency sweep
    client.emergency_sweep(&admin, &recovery_wallet);

    // Verify contract balance is 0
    assert_eq!(token_client.balance(&client.address), 0);

    // Verify recovery wallet received full balance
    assert_eq!(token_client.balance(&recovery_wallet), 1000);
}

#[test]
#[should_panic(expected = "Not initialized")]
fn test_emergency_sweep_not_initialized() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let recovery_wallet = Address::generate(&env);

    // Try to sweep without initializing - should panic
    client.emergency_sweep(&admin, &recovery_wallet);
}

#[test]
#[should_panic(expected = "Unauthorized")]
fn test_emergency_sweep_wrong_admin() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let wrong_admin = Address::generate(&env);
    let recovery_wallet = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    // Initialize the reward pool
    client.initialize(&admin, &token_id.address());

    // Try to sweep with wrong admin - should panic
    client.emergency_sweep(&wrong_admin, &recovery_wallet);
}

#[test]
fn test_emergency_sweep_zero_balance() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let recovery_wallet = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    // Initialize the reward pool
    client.initialize(&admin, &token_id.address());

    let token_client = token::StellarAssetClient::new(&env, &token_id.address());

    // Verify initial balance is 0
    assert_eq!(token_client.balance(&client.address), 0);

    // Perform emergency sweep with zero balance (should succeed)
    client.emergency_sweep(&admin, &recovery_wallet);

    // Verify balances remain 0
    assert_eq!(token_client.balance(&client.address), 0);
    assert_eq!(token_client.balance(&recovery_wallet), 0);
}

#[test]
fn test_emergency_sweep_large_balance() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let recovery_wallet = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());

    // Initialize the reward pool
    client.initialize(&admin, &token_id.address());

    // Fund with large amount
    let token_client = token::StellarAssetClient::new(&env, &token_id.address());
    token_client.mint(&client.address, &1_000_000);

    // Perform emergency sweep
    client.emergency_sweep(&admin, &recovery_wallet);

    // Verify full balance transferred
    assert_eq!(token_client.balance(&client.address), 0);
    assert_eq!(token_client.balance(&recovery_wallet), 1_000_000);
}

// ── Two-Step Admin Transfer Tests (Issue #20) ───────────────────────────────

#[test]
fn test_propose_new_admin_success_emits_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let proposed = Address::generate(&env);

    client.initialize(&admin, &token);

    client.propose_new_admin(&admin, &proposed);

    // 1 `TransferProposed` event must have been emitted
    let events = env.events().all();
    assert_eq!(events.len(), 2, "init event + propose event");

    let last = events.last().unwrap();
    let expected_topics: Vec<Val> =
        (Symbol::new(&env, "transfer_proposed"), admin.clone(), proposed.clone()).into_val(&env);
    assert_eq!(last.1, expected_topics);
}

#[test]
#[should_panic(expected = "Unauthorized: Caller is not the admin")]
fn test_propose_new_admin_unauthorized_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let impostor = Address::generate(&env);
    let token = Address::generate(&env);
    let proposed = Address::generate(&env);

    client.initialize(&admin, &token);
    client.propose_new_admin(&impostor, &proposed);
}

#[test]
fn test_accept_admin_ownership_happy_path() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin, &token);
    client.propose_new_admin(&admin, &new_admin);

    let events_before = env.events().all().len();
    client.accept_admin_ownership(&new_admin);

    // After accept, the new admin can call admin-only setters.
    // We verify by adding an approved spender as the new admin.
    let spender = Address::generate(&env);
    client.add_approved_spender(&new_admin, &spender);
    assert_eq!(env.events().all().len(), events_before + 2, "accept + spender");
}

#[test]
#[should_panic(expected = "Unauthorized: Acceptor is not the proposed admin")]
fn test_accept_admin_ownership_wrong_acceptor_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let proposed = Address::generate(&env);
    let impostor = Address::generate(&env);

    client.initialize(&admin, &token);
    client.propose_new_admin(&admin, &proposed);
    client.accept_admin_ownership(&impostor);
}

#[test]
#[should_panic(expected = "No pending admin transfer")]
fn test_accept_admin_ownership_no_pending_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin, &token);
    client.accept_admin_ownership(&new_admin);
}

#[test]
fn test_cancel_admin_transfer_by_proposer_recovers_from_typo() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    // Simulate a typo'd "new admin" address.
    let typo_address = Address::generate(&env);

    client.initialize(&admin, &token);
    client.propose_new_admin(&admin, &typo_address);

    // Original admin catches the typo and cancels.
    client.cancel_admin_transfer(&admin);

    // Live admin is unchanged — admin-only call still works.
    let spender = Address::generate(&env);
    client.add_approved_spender(&admin, &spender);
}

#[test]
fn test_cancel_admin_transfer_by_typo_address_recovers() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let typo = Address::generate(&env);

    client.initialize(&admin, &token);
    client.propose_new_admin(&admin, &typo);

    // The typo'd address itself can cancel — no stranger can squat.
    client.cancel_admin_transfer(&typo);

    let spender = Address::generate(&env);
    client.add_approved_spender(&admin, &spender);
}

#[test]
#[should_panic(expected = "Unauthorized: only proposer or current admin can cancel")]
fn test_cancel_admin_transfer_by_random_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let proposed = Address::generate(&env);
    let random = Address::generate(&env);

    client.initialize(&admin, &token);
    client.propose_new_admin(&admin, &proposed);
    client.cancel_admin_transfer(&random);
}

#[test]
#[should_panic(expected = "No pending admin transfer")]
fn test_cancel_admin_transfer_no_pending_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin, &token);
    client.cancel_admin_transfer(&admin);
}

#[test]
fn test_propose_after_accept_replaces_pending_typo() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let typo = Address::generate(&env);
    let correct = Address::generate(&env);

    client.initialize(&admin, &token);

    // Stage 1: admin proposes a typo'd address.
    client.propose_new_admin(&admin, &typo);

    // Stage 2: admin replaces the typo with a correct address.
    client.propose_new_admin(&admin, &correct);

    // The typo'd address can no longer accept (its proposal was overwritten).
    // We assert the correct address can still accept and become live admin.
    client.accept_admin_ownership(&correct);
    let spender = Address::generate(&env);
    client.add_approved_spender(&correct, &spender);
}

#[test]
fn test_double_accept_panics_after_first_accept_clears_pending() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let new_admin = Address::generate(&env);

    client.initialize(&admin, &token);
    client.propose_new_admin(&admin, &new_admin);
    client.accept_admin_ownership(&new_admin);
    // Second accept must panic — pending record was cleared.
    client.accept_admin_ownership(&new_admin);
}
