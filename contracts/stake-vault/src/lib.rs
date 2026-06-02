#![no_std]
use soroban_sdk::{contract, contractevent, contractimpl, contracttype, Address, BytesN, Env};

#[contracttype]
pub enum DataKey {
    UserStake(Address),
    Admin,
}

#[contracttype]
pub struct StakeInfo {
    pub amount: i128,
    pub lock_timestamp: u64,
}

#[contractevent]
pub struct ContractUpgraded {
    #[topic]
    pub admin: Address,
    pub new_wasm_hash: BytesN<32>,
}

#[contract]
pub struct StakeVault;

#[contractimpl]
impl StakeVault {
    /// Initializes the StakeVault contract with the protocol admin.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

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

    /// Upgrades the contract WASM. Only callable by the Protocol Admin.
    pub fn upgrade_contract(env: Env, admin: Address, new_wasm_hash: BytesN<32>) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        assert!(admin == stored_admin, "Unauthorized");

        env.deployer().update_current_contract_wasm(new_wasm_hash.clone());

        ContractUpgraded { admin, new_wasm_hash }.publish(&env);
    }
}

mod test;
