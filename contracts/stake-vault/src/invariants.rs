//! Invariant tests for the StakeVault contract.
//!
//! For every public method these tests assert:
//!
//! 1. **Event cardinality** — exactly the documented number of events is emitted.
//! 2. **Storage consistency** — `UserStake` slot shape matches the expected state
//!    (present with correct amount after stake, absent after unstake).
//! 3. **Token balance invariants** — value conservation across stake/unstake cycles.
//! 4. **Multiplier tier** — `get_multiplier` always returns a value consistent with
//!    the stored `StakeInfo.amount`.

#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Events, Ledger},
    token, Address, Env,
};

use crate::{
    types::{DataKey, StakeInfo},
    StakeVault, StakeVaultClient,
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

fn setup() -> (Env, StakeVaultClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(StakeVault, ());
    let client = StakeVaultClient::new(&env, &id);
    (env, client)
}

/// Returns an initialized vault with a funded SAC token.
fn setup_initialized(
    initial_user_balance: i128,
) -> (
    Env,
    StakeVaultClient<'static>,
    Address,         // user
    Address,         // token address
    token::StellarAssetClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(StakeVault, ());
    let client = StakeVaultClient::new(&env, &id);

    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let sac = token::StellarAssetClient::new(&env, &token_id);

    client.initialize(&admin, &token_id);

    let user = Address::generate(&env);
    sac.mint(&user, &initial_user_balance);

    (env, client, user, token_id, sac)
}

fn assert_user_stake_amount(env: &Env, contract: &Address, user: &Address, expected: i128) {
    env.as_contract(contract, || {
        let info: StakeInfo = env
            .storage()
            .persistent()
            .get(&DataKey::UserStake(user.clone()))
            .expect("invariant: UserStake slot must exist");
        assert_eq!(
            info.amount, expected,
            "UserStake({:?}).amount expected={}, got={}",
            user, expected, info.amount
        );
    });
}

fn assert_no_user_stake(env: &Env, contract: &Address, user: &Address) {
    env.as_contract(contract, || {
        let exists = env
            .storage()
            .persistent()
            .has(&DataKey::UserStake(user.clone()));
        assert!(
            !exists,
            "storage invariant violated: UserStake({:?}) must not exist after unstake",
            user
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

fn assert_token_stored(env: &Env, contract: &Address, expected: &Address) {
    env.as_contract(contract, || {
        let stored: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("invariant: DataKey::Token must be set");
        assert_eq!(stored, *expected);
    });
}

// ── initialize ────────────────────────────────────────────────────────────────

/// `initialize` must emit exactly **1** event (`StakeVaultInitialized`).
#[test]
fn inv_initialize_emits_exactly_one_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();

    client.initialize(&admin, &token_id);

    assert_event_count!(env, 1);
}

/// After `initialize`, `DataKey::Admin` and `DataKey::Token` must be stored correctly.
#[test]
fn inv_initialize_stores_admin_and_token() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();

    client.initialize(&admin, &token_id);

    assert_admin_stored(&env, &client.address, &admin);
    assert_token_stored(&env, &client.address, &token_id);
}

/// After `initialize`, no `UserStake` slots must exist (no orphaned storage).
#[test]
fn inv_initialize_no_orphan_user_stake() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let random_user = Address::generate(&env);

    client.initialize(&admin, &token_id);

    assert_no_user_stake(&env, &client.address, &random_user);
}

// ── stake ─────────────────────────────────────────────────────────────────────

/// `stake` must emit exactly **1** event (`Staked`).
#[test]
fn inv_stake_emits_exactly_one_event() {
    let (env, client, user, _token, _sac) = setup_initialized(500);

    env.ledger().set_timestamp(1_000_000);
    let baseline = env.events().all().len(); // after initialize

    client.stake(&user, &100);

    assert_eq!(events_since(&env, baseline), 1,
        "stake must emit exactly 1 event");
}

/// After `stake`, `UserStake(user).amount` must equal the staked amount.
#[test]
fn inv_stake_creates_user_stake_slot() {
    let (env, client, user, _token, _sac) = setup_initialized(500);

    env.ledger().set_timestamp(1_000_000);
    client.stake(&user, &200);

    assert_user_stake_amount(&env, &client.address, &user, 200);
}

/// Multiple stakes accumulate in `UserStake.amount`.
#[test]
fn inv_stake_accumulates_amount() {
    let (env, client, user, _token, _sac) = setup_initialized(500);

    env.ledger().set_timestamp(1_000_000);
    client.stake(&user, &100);
    assert_user_stake_amount(&env, &client.address, &user, 100);

    env.ledger().set_timestamp(1_000_100);
    client.stake(&user, &150);
    assert_user_stake_amount(&env, &client.address, &user, 250);
}

/// Token conservation: tokens staked must leave the user's wallet and arrive at the vault.
#[test]
fn inv_stake_token_conservation() {
    let amount = 300i128;
    let (env, client, user, token_id, sac) = setup_initialized(amount);

    let tok = token::Client::new(&env, &token_id);
    let user_before = tok.balance(&user);
    let vault_before = tok.balance(&client.address);

    env.ledger().set_timestamp(1_000_000);
    client.stake(&user, &amount);

    assert_eq!(user_before - tok.balance(&user), amount);
    assert_eq!(tok.balance(&client.address) - vault_before, amount);
    let _ = sac;
}

/// Each `stake` call emits exactly 1 event; N calls → N events total after init.
#[test]
fn inv_stake_event_count_linear() {
    let (env, client, user, _token, _sac) = setup_initialized(600);
    let baseline = env.events().all().len();

    env.ledger().set_timestamp(1_000_000);
    client.stake(&user, &100);
    assert_eq!(events_since(&env, baseline), 1);

    env.ledger().set_timestamp(1_000_100);
    client.stake(&user, &100);
    assert_eq!(events_since(&env, baseline), 2);

    env.ledger().set_timestamp(1_000_200);
    client.stake(&user, &100);
    assert_eq!(events_since(&env, baseline), 3);
}

/// Staking for user A must not create a `UserStake` slot for user B.
#[test]
fn inv_stake_no_cross_user_storage_leak() {
    let (env, client, user_a, token_id, sac) = setup_initialized(500);
    let user_b = Address::generate(&env);

    env.ledger().set_timestamp(1_000_000);
    client.stake(&user_a, &100);

    assert_no_user_stake(&env, &client.address, &user_b);
    let _ = (token_id, sac);
}

// ── unstake ───────────────────────────────────────────────────────────────────

const LOCK: u64 = 604_800; // DEFAULT_LOCK_PERIOD_SECONDS

/// `unstake` must emit exactly **1** event (`Unstaked`).
#[test]
fn inv_unstake_emits_exactly_one_event() {
    let (env, client, user, _token, _sac) = setup_initialized(500);

    env.ledger().set_timestamp(1_000_000);
    client.stake(&user, &100);
    let baseline = env.events().all().len();

    env.ledger().set_timestamp(1_000_000 + LOCK);
    client.unstake(&user);

    assert_eq!(events_since(&env, baseline), 1,
        "unstake must emit exactly 1 event");
}

/// After `unstake`, `UserStake(user)` must be **deleted** from persistent storage.
#[test]
fn inv_unstake_removes_user_stake_slot() {
    let (env, client, user, _token, _sac) = setup_initialized(500);

    env.ledger().set_timestamp(2_000_000);
    client.stake(&user, &200);

    env.ledger().set_timestamp(2_000_000 + LOCK);
    client.unstake(&user);

    assert_no_user_stake(&env, &client.address, &user);
}

/// Token conservation: unstaked tokens must return to the user's wallet exactly.
#[test]
fn inv_unstake_token_conservation() {
    let amount = 400i128;
    let (env, client, user, token_id, sac) = setup_initialized(amount);

    let tok = token::Client::new(&env, &token_id);

    env.ledger().set_timestamp(3_000_000);
    client.stake(&user, &amount);

    let user_after_stake = tok.balance(&user);
    let vault_after_stake = tok.balance(&client.address);

    env.ledger().set_timestamp(3_000_000 + LOCK);
    client.unstake(&user);

    assert_eq!(tok.balance(&user) - user_after_stake, amount);
    assert_eq!(vault_after_stake - tok.balance(&client.address), amount);
    let _ = sac;
}

/// Admin and token storage slots must survive the stake → unstake cycle intact.
#[test]
fn inv_unstake_config_slots_intact() {
    let (env, client, user, token_id, sac) = setup_initialized(500);
    let admin = Address::generate(&env); // we don't re-register; we check stored values

    // Re-read what initialize actually stored
    let stored_admin = env.as_contract(&client.address, || {
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .unwrap()
    });
    let stored_token = env.as_contract(&client.address, || {
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Token)
            .unwrap()
    });

    env.ledger().set_timestamp(4_000_000);
    client.stake(&user, &100);
    env.ledger().set_timestamp(4_000_000 + LOCK);
    client.unstake(&user);

    assert_admin_stored(&env, &client.address, &stored_admin);
    assert_token_stored(&env, &client.address, &stored_token);
    let _ = (admin, token_id, sac);
}

// ── get_multiplier (pure read) ────────────────────────────────────────────────

/// `get_multiplier` must emit **0** events regardless of stake level.
#[test]
fn inv_get_multiplier_emits_no_events() {
    let (env, client, user, _token, _sac) = setup_initialized(1_000);

    env.ledger().set_timestamp(5_000_000);
    client.stake(&user, &500);

    let baseline = env.events().all().len();
    let _ = client.get_multiplier(&user);
    assert_eq!(events_since(&env, baseline), 0,
        "get_multiplier must not emit any events");
}

/// `get_multiplier` tier boundaries must be consistent with stored stake amount.
///
/// | amount  | expected BPS |
/// |---------|-------------|
/// | 0       | 100          |
/// | 99      | 100          |
/// | 100     | 120          |
/// | 499     | 120          |
/// | 500     | 200          |
/// | 1000    | 200          |
#[test]
fn inv_get_multiplier_tiers_consistent_with_stored_amount() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone()).address();
    client.initialize(&admin, &token_id);

    let user = Address::generate(&env);

    let cases: &[(i128, u32)] = &[
        (0, 100),
        (50, 100),
        (99, 100),
        (100, 120),
        (499, 120),
        (500, 200),
        (1_000, 200),
    ];

    for &(amount, expected_bps) in cases {
        env.as_contract(&client.address, || {
            if amount == 0 {
                // Remove the key so get_multiplier sees no stake
                env.storage()
                    .persistent()
                    .remove(&DataKey::UserStake(user.clone()));
            } else {
                env.storage().persistent().set(
                    &DataKey::UserStake(user.clone()),
                    &StakeInfo { amount, lock_timestamp: 0 },
                );
            }
        });

        let bps = client.get_multiplier(&user);
        assert_eq!(
            bps, expected_bps,
            "get_multiplier invariant: amount={} expected={} got={}",
            amount, expected_bps, bps
        );
    }
}
