#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

use crate::{
    types::{DataKey, StakeInfo},
    StakeVault, StakeVaultClient,
};

fn setup() -> (Env, StakeVaultClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(StakeVault, ());
    let client = StakeVaultClient::new(&env, &contract_id);
    (env, client)
}

#[test]
fn test_stake_transfers_and_accumulates() {
    let (env, client) = setup();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    let token_id = env.register_stellar_asset_contract_v2(admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_id.address());

    client.initialize(&admin, &token_id.address());

    token_client.mint(&user, &1000);

    env.ledger().set_timestamp(1_000_000);
    client.stake(&user, &100);

    assert_eq!(token_client.balance(&user), 900);
    assert_eq!(token_client.balance(&client.address), 100);

    env.ledger().set_timestamp(1_000_100);
    client.stake(&user, &50);

    assert_eq!(token_client.balance(&user), 850);
    assert_eq!(token_client.balance(&client.address), 150);

    env.ledger().set_timestamp(1_000_100 + 604800);
    client.unstake(&user);

    assert_eq!(token_client.balance(&user), 1000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
#[should_panic(expected = "Lock period active")]
fn test_stake_resets_lock_timestamp() {
    let (env, client) = setup();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    let token_id = env.register_stellar_asset_contract_v2(admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_id.address());

    client.initialize(&admin, &token_id.address());
    token_client.mint(&user, &1000);

    env.ledger().set_timestamp(10_000);
    client.stake(&user, &100);

    env.ledger().set_timestamp(10_100);
    client.stake(&user, &50);

    env.ledger().set_timestamp(10_000 + 604800);
    client.unstake(&user);
}

#[test]
#[should_panic(expected = "Lock period active")]
fn test_unstake_panics_when_lock_period_active() {
    let (env, client) = setup();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    let token_id = env.register_stellar_asset_contract_v2(admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_id.address());

    client.initialize(&admin, &token_id.address());
    token_client.mint(&user, &1000);

    env.ledger().set_timestamp(2_000_000);
    client.stake(&user, &100);

    env.ledger().set_timestamp(2_000_000 + 604799);
    client.unstake(&user);
}

#[test]
fn test_unstake_succeeds_after_lock() {
    let (env, client) = setup();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    let token_id = env.register_stellar_asset_contract_v2(admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_id.address());

    client.initialize(&admin, &token_id.address());
    token_client.mint(&user, &1000);

    env.ledger().set_timestamp(3_000_000);
    client.stake(&user, &250);

    env.ledger().set_timestamp(3_000_000 + 604800);
    client.unstake(&user);

    assert_eq!(token_client.balance(&user), 1000);
    assert_eq!(token_client.balance(&client.address), 0);
}

#[test]
#[should_panic(expected = "No stake found")]
fn test_unstake_twice_panics_after_withdrawal() {
    let (env, client) = setup();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);

    let token_id = env.register_stellar_asset_contract_v2(admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_id.address());

    client.initialize(&admin, &token_id.address());
    token_client.mint(&user, &1000);

    env.ledger().set_timestamp(4_000_000);
    client.stake(&user, &100);

    env.ledger().set_timestamp(4_000_000 + 604800);
    client.unstake(&user);

    client.unstake(&user);
}

#[test]
fn test_get_multiplier() {
    let (env, client) = setup();
    let user = Address::generate(&env);

    assert_eq!(client.get_multiplier(&user), 100);

    env.as_contract(&client.address, || {
        env.storage().persistent().set(
            &DataKey::UserStake(user.clone()),
            &StakeInfo {
                amount: 50,
                lock_timestamp: 0,
            },
        );
    });
    assert_eq!(client.get_multiplier(&user), 100);

    env.as_contract(&client.address, || {
        env.storage().persistent().set(
            &DataKey::UserStake(user.clone()),
            &StakeInfo {
                amount: 100,
                lock_timestamp: 0,
            },
        );
    });
    assert_eq!(client.get_multiplier(&user), 120);

    env.as_contract(&client.address, || {
        env.storage().persistent().set(
            &DataKey::UserStake(user.clone()),
            &StakeInfo {
                amount: 499,
                lock_timestamp: 0,
            },
        );
    });
    assert_eq!(client.get_multiplier(&user), 120);

    env.as_contract(&client.address, || {
        env.storage().persistent().set(
            &DataKey::UserStake(user.clone()),
            &StakeInfo {
                amount: 500,
                lock_timestamp: 0,
            },
        );
    });
    assert_eq!(client.get_multiplier(&user), 200);

    env.as_contract(&client.address, || {
        env.storage().persistent().set(
            &DataKey::UserStake(user.clone()),
            &StakeInfo {
                amount: 1000,
                lock_timestamp: 0,
            },
        );
    });
    assert_eq!(client.get_multiplier(&user), 200);
}

// ── Storage versioning & migrations ──────────────────────────────────────────

/// A freshly deployed contract reports version 0 (no Version key set yet).
#[test]
fn test_stake_vault_contract_version_initial_zero() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());
    client.initialize(&admin, &token_id.address());

    assert_eq!(client.contract_version(), 0);
}

/// After `migrate()` the stored version equals the compiled VERSION constant.
#[test]
fn test_stake_vault_migrate_v0_to_v1_sets_version() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());
    client.initialize(&admin, &token_id.address());

    assert_eq!(client.contract_version(), 0);
    client.migrate(&admin);
    assert_eq!(client.contract_version(), crate::VERSION);
}

/// `migrate()` called twice panics with "Already at current version".
#[test]
#[should_panic(expected = "Already at current version")]
fn test_stake_vault_migrate_twice_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());
    client.initialize(&admin, &token_id.address());

    client.migrate(&admin);
    client.migrate(&admin);
}

/// Non-admin cannot call `migrate()`.
#[test]
#[should_panic(expected = "Unauthorized")]
fn test_stake_vault_migrate_unauthorized_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());
    client.initialize(&admin, &token_id.address());

    client.migrate(&attacker);
}

/// Stake records written before migration are intact afterwards (v0 → v1).
#[test]
fn test_stake_vault_migrate_v0_to_v1_preserves_stakes() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_id.address());

    client.initialize(&admin, &token_id.address());
    token_client.mint(&user, &500);
    client.stake(&user, &200i128);

    let multiplier_before = client.get_multiplier(&user);
    assert_eq!(client.contract_version(), 0);

    client.migrate(&admin);
    assert_eq!(client.contract_version(), crate::VERSION);

    // Stake data must be intact — multiplier depends on stored amount.
    assert_eq!(client.get_multiplier(&user), multiplier_before);
}
