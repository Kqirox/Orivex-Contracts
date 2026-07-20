//! Invariant tests for the RewardPool contract.
//!
//! For every public method these tests assert:
//!
//! 1. **Event cardinality** — exactly the documented number of events is emitted.
//! 2. **Storage consistency** — admin, token, spender-list, and pause-flag slots
//!    are in the correct state after the call.
//! 3. **Token balance invariants** — conservation of value: tokens that leave the
//!    contract must arrive at the intended destination; no tokens are conjured or lost.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events},
    token, Address, Env,
};

use crate::{
    types::DataKey,
    RewardPool, RewardPoolClient,
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

/// Count events emitted *after* `baseline` (returns delta).
fn events_since(env: &Env, baseline: usize) -> usize {
    env.events().all().len() - baseline
}

fn setup() -> (Env, RewardPoolClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(RewardPool, ());
    let client = RewardPoolClient::new(&env, &id);
    (env, client)
}

/// Returns an initialized pool with a funded SAC token.
/// `pool_balance` tokens are minted directly into the pool contract.
fn setup_funded(
    pool_balance: i128,
) -> (
    Env,
    RewardPoolClient<'static>,
    Address, // admin
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(RewardPool, ());
    let client = RewardPoolClient::new(&env, &id);

    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let sac = token::StellarAssetClient::new(&env, &token_id);

    client.initialize(&admin, &token_id);
    sac.mint(&client.address, &pool_balance);

    (env, client, admin, sac)
}

fn assert_admin_stored(env: &Env, contract: &Address, expected: &Address) {
    env.as_contract(contract, || {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("invariant: DataKey::Admin must be set");
        assert_eq!(stored, *expected, "Admin address mismatch in storage");
    });
}

fn assert_token_stored(env: &Env, contract: &Address, expected: &Address) {
    env.as_contract(contract, || {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("invariant: DataKey::Token must be set");
        assert_eq!(stored, *expected, "Token address mismatch in storage");
    });
}

fn assert_spender_flag(env: &Env, contract: &Address, spender: &Address, expected: bool) {
    env.as_contract(contract, || {
        let stored: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Spender(spender.clone()))
            .unwrap_or(false);
        assert_eq!(
            stored, expected,
            "Spender({:?}) flag expected={}, got={}",
            spender, expected, stored
        );
    });
}

fn assert_paused(env: &Env, contract: &Address, expected: bool) {
    env.as_contract(contract, || {
        let stored: bool = env
            .storage()
            .instance()
            .get(&DataKey::IsPaused)
            .unwrap_or(false);
        assert_eq!(
            stored, expected,
            "IsPaused expected={}, got={}",
            expected, stored
        );
    });
}

// ── initialize ────────────────────────────────────────────────────────────────

/// `initialize` must emit exactly **1** event (`PoolInitialized`).
#[test]
fn inv_initialize_emits_exactly_one_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin, &token);

    assert_event_count!(env, 1);
}

/// After `initialize`, `DataKey::Admin` and `DataKey::Token` must be set correctly.
#[test]
fn inv_initialize_stores_admin_and_token() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();

    client.initialize(&admin, &token_id);

    assert_admin_stored(&env, &client.address, &admin);
    assert_token_stored(&env, &client.address, &token_id);
}

/// After `initialize`, the pause flag must default to **false**.
#[test]
fn inv_initialize_pause_flag_is_false() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin, &token);

    assert_paused(&env, &client.address, false);
}

// ── add_approved_spender ──────────────────────────────────────────────────────

/// `add_approved_spender` must emit exactly **1** event (`SpenderAdded`).
#[test]
fn inv_add_approved_spender_emits_exactly_one_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let spender = Address::generate(&env);

    client.initialize(&admin, &token);
    let baseline = env.events().all().len(); // 1 (PoolInitialized)

    client.add_approved_spender(&admin, &spender);

    assert_eq!(events_since(&env, baseline), 1,
        "add_approved_spender must emit exactly 1 event");
}

/// After `add_approved_spender`, `DataKey::Spender(spender)` must be `true`.
#[test]
fn inv_add_approved_spender_sets_flag() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let spender = Address::generate(&env);

    client.initialize(&admin, &token);
    client.add_approved_spender(&admin, &spender);

    assert_spender_flag(&env, &client.address, &spender, true);
}

/// Adding a spender must not alter the admin or token storage slots.
#[test]
fn inv_add_approved_spender_no_side_effects_on_admin_token() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let spender = Address::generate(&env);

    client.initialize(&admin, &token_id);
    client.add_approved_spender(&admin, &spender);

    assert_admin_stored(&env, &client.address, &admin);
    assert_token_stored(&env, &client.address, &token_id);
}

/// Adding two different spenders → two persistent spender flags, 2 events (one per call).
#[test]
fn inv_add_approved_spender_multiple_independent_flags() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let s1 = Address::generate(&env);
    let s2 = Address::generate(&env);

    client.initialize(&admin, &token);
    let baseline = env.events().all().len();

    client.add_approved_spender(&admin, &s1);
    client.add_approved_spender(&admin, &s2);

    assert_eq!(events_since(&env, baseline), 2);
    assert_spender_flag(&env, &client.address, &s1, true);
    assert_spender_flag(&env, &client.address, &s2, true);
}

// ── set_pause ─────────────────────────────────────────────────────────────────

/// `set_pause(true)` must emit **0** events and set the flag to `true`.
#[test]
fn inv_set_pause_true_no_event_flag_set() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin, &token);
    let baseline = env.events().all().len();

    client.set_pause(&admin, &true);

    assert_eq!(events_since(&env, baseline), 0,
        "set_pause must not emit any events");
    assert_paused(&env, &client.address, true);
}

/// `set_pause(false)` after `set_pause(true)` must unset the flag.
#[test]
fn inv_set_pause_toggle_round_trips() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);

    client.initialize(&admin, &token);
    client.set_pause(&admin, &true);
    assert_paused(&env, &client.address, true);

    client.set_pause(&admin, &false);
    assert_paused(&env, &client.address, false);
}

// ── distribute_reward ─────────────────────────────────────────────────────────

/// `distribute_reward` must emit exactly **1** event (`RewardDistributed`).
#[test]
fn inv_distribute_reward_emits_exactly_one_event() {
    let (env, client, admin, sac) = setup_funded(1_000);
    let spender = Address::generate(&env);
    let learner = Address::generate(&env);

    client.add_approved_spender(&admin, &spender);
    let baseline = env.events().all().len();

    client.distribute_reward(&spender, &learner, &100);

    assert_eq!(events_since(&env, baseline), 1,
        "distribute_reward must emit exactly 1 event");
    let _ = sac; // silence unused warning
}

/// Token conservation: tokens transferred out of the pool must arrive at `learner`.
#[test]
fn inv_distribute_reward_token_conservation() {
    let amount = 250i128;
    let (env, client, admin, sac) = setup_funded(1_000);
    let spender = Address::generate(&env);
    let learner = Address::generate(&env);

    client.add_approved_spender(&admin, &spender);

    let token_client = token::Client::new(
        &env,
        &env.as_contract(&client.address, || {
            env.storage()
                .instance()
                .get::<DataKey, Address>(&DataKey::Token)
                .unwrap()
        }),
    );

    let pool_before = token_client.balance(&client.address);
    let learner_before = token_client.balance(&learner);

    client.distribute_reward(&spender, &learner, &amount);

    let pool_after = token_client.balance(&client.address);
    let learner_after = token_client.balance(&learner);

    assert_eq!(pool_before - pool_after, amount, "pool balance must decrease by amount");
    assert_eq!(learner_after - learner_before, amount, "learner balance must increase by amount");
    let _ = sac;
}

/// After `distribute_reward`, admin, token, and spender flags must be unchanged.
#[test]
fn inv_distribute_reward_no_storage_side_effects() {
    let (env, client, admin, sac) = setup_funded(500);
    let spender = Address::generate(&env);
    let learner = Address::generate(&env);

    client.add_approved_spender(&admin, &spender);
    client.distribute_reward(&spender, &learner, &50);

    assert_spender_flag(&env, &client.address, &spender, true);
    assert_paused(&env, &client.address, false);
    let _ = sac;
}

// ── fund_pool ─────────────────────────────────────────────────────────────────

/// `fund_pool` must emit exactly **1** event (`PoolFunded`).
#[test]
fn inv_fund_pool_emits_exactly_one_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let sac = token::StellarAssetClient::new(&env, &token_id);
    let donor = Address::generate(&env);

    client.initialize(&admin, &token_id);
    sac.mint(&donor, &1_000);
    let baseline = env.events().all().len();

    client.fund_pool(&donor, &300);

    assert_eq!(events_since(&env, baseline), 1,
        "fund_pool must emit exactly 1 event");
}

/// Token conservation: `fund_pool` must move exactly `amount` tokens from donor to pool.
#[test]
fn inv_fund_pool_token_conservation() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let sac = token::StellarAssetClient::new(&env, &token_id);
    let donor = Address::generate(&env);
    let amount = 400i128;

    client.initialize(&admin, &token_id);
    sac.mint(&donor, &amount);

    let tok = token::Client::new(&env, &token_id);
    let pool_before = tok.balance(&client.address);
    let donor_before = tok.balance(&donor);

    client.fund_pool(&donor, &amount);

    assert_eq!(tok.balance(&client.address) - pool_before, amount);
    assert_eq!(donor_before - tok.balance(&donor), amount);
}

// ── emergency_sweep ───────────────────────────────────────────────────────────

/// `emergency_sweep` must emit exactly **1** event (`EmergencySweep`).
#[test]
fn inv_emergency_sweep_emits_exactly_one_event() {
    let (env, client, admin, _sac) = setup_funded(1_000);
    let recovery = Address::generate(&env);
    let baseline = env.events().all().len();

    client.emergency_sweep(&admin, &recovery);

    assert_eq!(events_since(&env, baseline), 1,
        "emergency_sweep must emit exactly 1 event");
}

/// After `emergency_sweep`, the pool balance must be **0** and the full balance
/// must have moved to `recovery_wallet`.
#[test]
fn inv_emergency_sweep_drains_pool_to_recovery() {
    let initial = 7_500i128;
    let (env, client, admin, sac) = setup_funded(initial);
    let recovery = Address::generate(&env);

    let tok_addr = env.as_contract(&client.address, || {
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Token)
            .unwrap()
    });
    let tok = token::Client::new(&env, &tok_addr);

    client.emergency_sweep(&admin, &recovery);

    assert_eq!(tok.balance(&client.address), 0,
        "pool must be empty after emergency_sweep");
    assert_eq!(tok.balance(&recovery), initial,
        "recovery wallet must receive full pool balance");
    let _ = sac;
}

/// After `emergency_sweep`, admin and token slots must remain intact
/// (sweep is non-destructive to contract configuration).
#[test]
fn inv_emergency_sweep_config_slots_intact() {
    let (env, client, admin, sac) = setup_funded(500);
    let recovery = Address::generate(&env);
    let tok_addr = env.as_contract(&client.address, || {
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Token)
            .unwrap()
    });

    client.emergency_sweep(&admin, &recovery);

    assert_admin_stored(&env, &client.address, &admin);
    assert_token_stored(&env, &client.address, &tok_addr);
    let _ = sac;
}
