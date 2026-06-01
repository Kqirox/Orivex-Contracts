#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    UserStake(Address),
}

#[contracttype]
pub struct StakeInfo {
    pub amount: i128,
    pub lock_timestamp: u64,
}

#[contract]
pub struct StakeVault;

#[contractimpl]
impl StakeVault {
    pub fn get_multiplier(env: Env, user: Address) -> u32 {
        let stake_info = env.storage().persistent().get(&DataKey::UserStake(user)).unwrap_or(StakeInfo { amount: 0, lock_timestamp: 0 });
        if stake_info.amount >= 500 {
            200
        } else if stake_info.amount >= 100 {
            120
        } else {
            100
        }
    }
}

mod test;
