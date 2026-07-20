#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env,
};

use crate::{
    types::{DataKey, MultiplierBps, StakeInfo},
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

    assert_eq!(client.get_multiplier(&user), MultiplierBps::None);

    env.as_contract(&client.address, || {
        env.storage().persistent().set(
            &DataKey::UserStake(user.clone()),
            &StakeInfo {
                amount: 50,
                lock_timestamp: 0,
            },
        );
    });
    assert_eq!(client.get_multiplier(&user), MultiplierBps::None);

    env.as_contract(&client.address, || {
        env.storage().persistent().set(
            &DataKey::UserStake(user.clone()),
            &StakeInfo {
                amount: 100,
                lock_timestamp: 0,
            },
        );
    });
    assert_eq!(client.get_multiplier(&user), MultiplierBps::Low);

    env.as_contract(&client.address, || {
        env.storage().persistent().set(
            &DataKey::UserStake(user.clone()),
            &StakeInfo {
                amount: 499,
                lock_timestamp: 0,
            },
        );
    });
    assert_eq!(client.get_multiplier(&user), MultiplierBps::Low);

    env.as_contract(&client.address, || {
        env.storage().persistent().set(
            &DataKey::UserStake(user.clone()),
            &StakeInfo {
                amount: 500,
                lock_timestamp: 0,
            },
        );
    });
    assert_eq!(client.get_multiplier(&user), MultiplierBps::High);

    env.as_contract(&client.address, || {
        env.storage().persistent().set(
            &DataKey::UserStake(user.clone()),
            &StakeInfo {
                amount: 1000,
                lock_timestamp: 0,
            },
        );
    });
    assert_eq!(client.get_multiplier(&user), MultiplierBps::High);
}

#[test]
fn test_multiplier_bps_round_trip() {
    // Each variant must preserve its documented basis-points value through as_bps().
    assert_eq!(MultiplierBps::None.as_bps(), 100, "None tier must be 100 bps (1.0×)");
    assert_eq!(MultiplierBps::Low.as_bps(), 120, "Low tier must be 120 bps (1.2×)");
    assert_eq!(MultiplierBps::High.as_bps(), 200, "High tier must be 200 bps (2.0×)");

    // Scaled-payout arithmetic using as_bps() must match known-good values.
    let base: i128 = 850;
    assert_eq!((base * MultiplierBps::None.as_bps() as i128) / 100, 850);
    assert_eq!((base * MultiplierBps::Low.as_bps() as i128) / 100, 1020);
    assert_eq!((base * MultiplierBps::High.as_bps() as i128) / 100, 1700);
}
