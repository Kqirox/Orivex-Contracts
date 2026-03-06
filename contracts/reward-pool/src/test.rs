use soroban_sdk::{
    contracttype,
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env, IntoVal,
};

use crate::{PoolInitialized, RewardPool};
use crate::types::DataKey;

#[contractclient(name = "RewardPoolClient")]
impl RewardPool {}

#[test]
fn test_initialize_success() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RewardPool);
    let client = RewardPoolClient::new(&env, &contract_id);

    let admin = Address::random(&env);
    let token = Address::random(&env);

    // Initialize the contract
    client.initialize(&admin, &token);

    // Verify admin is stored
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap();
    assert_eq!(stored_admin, admin);

    // Verify token is stored
    let stored_token: Address = env
        .storage()
        .instance()
        .get(&DataKey::Token)
        .unwrap();
    assert_eq!(stored_token, token);

    // Verify event was emitted
    let events = env.events().all();
    assert_eq!(events.len(), 1);
    
    let event = &events[0];
    assert_eq!(event.contract_id, contract_id);
    assert_eq!(
        event.topics,
        vec![
            PoolInitialized::XDR_TYPE_NAME.into_val(&env),
            admin.into_val(&env),
            token.into_val(&env)
        ]
    );
}

#[test]
#[should_panic(expected = "Already initialized")]
fn test_initialize_twice_panics() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RewardPool);
    let client = RewardPoolClient::new(&env, &contract_id);

    let admin = Address::random(&env);
    let token = Address::random(&env);

    // First initialization should succeed
    client.initialize(&admin, &token);

    // Second initialization should panic
    client.initialize(&admin, &token);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #4)")]
fn test_initialize_without_auth_panics() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RewardPool);
    let client = RewardPoolClient::new(&env, &contract_id);

    let admin = Address::random(&env);
    let token = Address::random(&env);

    // Try to initialize without authentication - should panic
    client.initialize(&admin, &token);
}

#[test]
fn test_initialize_with_auth() {
    let env = Env::default();
    let contract_id = env.register_contract(None, RewardPool);
    let client = RewardPoolClient::new(&env, &contract_id);

    let admin = Address::random(&env);
    let token = Address::random(&env);

    // Mock authentication by setting the admin as the current contract caller
    env.mock_auths(&[
        (&admin, &contract_id.into_val(&env), &"initialize".into_val(&env))
    ]);

    // Initialize with proper authentication
    client.initialize(&admin, &token);

    // Verify storage is set correctly
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap();
    assert_eq!(stored_admin, admin);

    let stored_token: Address = env
        .storage()
        .instance()
        .get(&DataKey::Token)
        .unwrap();
    assert_eq!(stored_token, token);
}
