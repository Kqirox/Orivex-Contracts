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

// ── Two-Step Admin Transfer Tests (Issue #20) ────────────────────────────

#[test]
fn test_propose_new_admin_emits_event() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let proposed = Address::generate(&env);

    client.initialize(&admin, &token);
    client.propose_new_admin(&admin, &proposed);

    let events = env.events().all();
    assert_eq!(events.len(), 2, "init event + propose event");
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
    client.accept_admin_ownership(&new_admin);

    // After accept, the new admin can perform admin-only operations.
    // `upgrade_contract` is admin-only; supply a dummy wasm hash.
    let wasm = soroban_sdk::BytesN::from_array(&env, &[0u8; 32]);
    client.upgrade_contract(&new_admin, &wasm);
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
    let impostor = Address::generate(&env);

    client.initialize(&admin, &token);
    client.accept_admin_ownership(&impostor);
}

#[test]
fn test_cancel_admin_transfer_typo_recovery() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let typo = Address::generate(&env);

    client.initialize(&admin, &token);
    client.propose_new_admin(&admin, &typo);
    client.cancel_admin_transfer(&admin);

    // Admin authority is unchanged.
    let wasm = soroban_sdk::BytesN::from_array(&env, &[0u8; 32]);
    client.upgrade_contract(&admin, &wasm);
}

#[test]
fn test_cancel_admin_transfer_by_typo_self_recovery() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let typo = Address::generate(&env);

    client.initialize(&admin, &token);
    client.propose_new_admin(&admin, &typo);
    client.cancel_admin_transfer(&typo);

    let wasm = soroban_sdk::BytesN::from_array(&env, &[0u8; 32]);
    client.upgrade_contract(&admin, &wasm);
}

#[test]
#[should_panic(
    expected = "Unauthorized: only proposer or current admin can cancel"
)]
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
