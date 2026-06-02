#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_get_multiplier() {
    let env = Env::default();
    let contract_id = env.register_contract(None, StakeVault);
    let client = StakeVaultClient::new(&env, &contract_id);
    let user = Address::generate(&env);

    // Initial state (no stake)
    assert_eq!(client.get_multiplier(&user), 100);

    // Set stake to 50 (< 100)
    let stake_info = StakeInfo { amount: 50, lock_timestamp: 0 };
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&DataKey::UserStake(user.clone()), &stake_info);
    });
    assert_eq!(client.get_multiplier(&user), 100);

    // Set stake to 100 (100 <= x < 500)
    let stake_info = StakeInfo { amount: 100, lock_timestamp: 0 };
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&DataKey::UserStake(user.clone()), &stake_info);
    });
    assert_eq!(client.get_multiplier(&user), 120);

    // Set stake to 499 (100 <= x < 500)
    let stake_info = StakeInfo { amount: 499, lock_timestamp: 0 };
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&DataKey::UserStake(user.clone()), &stake_info);
    });
    assert_eq!(client.get_multiplier(&user), 120);

    // Set stake to 500 (x >= 500)
    let stake_info = StakeInfo { amount: 500, lock_timestamp: 0 };
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&DataKey::UserStake(user.clone()), &stake_info);
    });
    assert_eq!(client.get_multiplier(&user), 200);

    // Set stake to 1000 (x >= 500)
    let stake_info = StakeInfo { amount: 1000, lock_timestamp: 0 };
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&DataKey::UserStake(user.clone()), &stake_info);
    });
    assert_eq!(client.get_multiplier(&user), 200);
}
