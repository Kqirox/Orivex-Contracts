#![no_std]

pub const REWARD_TOKEN_DECIMALS: u32 = 7;

pub const MAX_SPENDERS: u32 = 256;
// Operational notes — the `IsPaused` flag is a global switch
// for `distribute_reward`. Fund recovery always routes via
// `emergency_sweep`, never via direct token transfer. Spender
// list entries are stored in persistent storage and persist
// across upgrades.

pub const MIN_PAYOUT_AMOUNT: i128 = 1;

pub const PLATFORM_FEE_BASIS_POINTS: u32 = 1500;
// Crate overview — central USDC reward distribution. Holds the
// reward-token balance and gates payouts behind an approved-
// spender allowlist.
use soroban_sdk::{contractclient, contractevent, Address, BytesN, Env};

pub mod types;

use orivex_common::bump_persistent;

#[contractclient(name = "RewardPoolClient")]
pub trait RewardPoolInterface {
    fn initialize(env: Env, admin: Address, token: Address);
    fn add_approved_spender(env: Env, admin: Address, spender: Address);
    fn set_pause(env: Env, admin: Address, status: bool);
    fn distribute_reward(env: Env, caller: Address, learner: Address, amount: i128);
    fn fund_pool(env: Env, donor: Address, amount: i128);
    fn emergency_sweep(env: Env, admin: Address, recovery_wallet: Address);
    fn upgrade_contract(env: Env, admin: Address, new_wasm_hash: BytesN<32>);
}

#[contractevent]
pub struct PoolInitialized {
    #[topic]
    pub admin: Address,
    #[topic]
    pub token: Address,
}

#[contractevent]
pub struct SpenderAdded {
    #[topic]
    pub spender: Address,
}

#[contractevent]
pub struct RewardDistributed {
    #[topic]
    pub caller: Address,
    #[topic]
    pub learner: Address,
    pub amount: i128,
}

#[contractevent]
pub struct PoolFunded {
    #[topic]
    pub donor: Address,
    pub amount: i128,
}

#[contractevent]
pub struct EmergencySweep {
    #[topic]
    pub admin: Address,
    #[topic]
    pub recovery_wallet: Address,
    pub amount: i128,
}

#[contractevent]
pub struct ContractUpgraded {
    #[topic]
    pub admin: Address,
    pub new_wasm_hash: BytesN<32>,
}

#[cfg(feature = "contract")]
mod contract_impl {
    use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env};

    use crate::types::DataKey;
    use crate::{
        bump_persistent, ContractUpgraded, EmergencySweep, PoolFunded, PoolInitialized,
        RewardDistributed, SpenderAdded,
    };

    #[contract]
    pub struct RewardPool;

    #[contractimpl]
    impl RewardPool {
        /// Initializes the RewardPool contract with admin and token addresses.
        ///
        /// # Panics
        /// * If contract is already initialized
        pub fn initialize(env: Env, admin: Address, token: Address) {
            if env.storage().instance().has(&DataKey::Admin) {
                panic!("Already initialized");
            }
            admin.require_auth();
            env.storage().instance().set(&DataKey::Admin, &admin);
            env.storage().instance().set(&DataKey::Token, &token);
            PoolInitialized { admin, token }.publish(&env);
        }

        /// Adds a contract address to the approved spender whitelist.
        ///
        /// # Panics
        /// * If contract is not initialized
        /// * If admin does not match stored admin
        /// * If admin authentication fails
        pub fn add_approved_spender(env: Env, admin: Address, spender: Address) {
            let stored_admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .expect("Not initialized");

            if admin != stored_admin {
                panic!("Unauthorized");
            }

            admin.require_auth();

            let spender_key = DataKey::Spender(spender.clone());
            env.storage()
                .persistent()
                .set(&spender_key, &true);
            bump_persistent(&env, &spender_key);

            SpenderAdded { spender }.publish(&env);
        }

        /// Toggles the pause state of the contract (emergency circuit breaker).
        ///
        /// # Panics
        /// * If contract is not initialized
        /// * If admin does not match stored admin
        pub fn set_pause(env: Env, admin: Address, status: bool) {
            let stored_admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .expect("Not initialized");

            if admin != stored_admin {
                panic!("Unauthorized");
            }

            admin.require_auth();

            env.storage().instance().set(&DataKey::IsPaused, &status);
        }

        /// Distributes rewards from the pool to a learner.
        ///
        /// # Panics
        /// * If caller authentication fails
        /// * If amount is not positive
        /// * If caller is not an authorized spender
        /// * If contract is not initialized
        pub fn distribute_reward(env: Env, caller: Address, learner: Address, amount: i128) {
            let is_paused: bool = env
                .storage()
                .instance()
                .get(&DataKey::IsPaused)
                .unwrap_or(false);
            assert!(!is_paused, "Contract is paused");

            caller.require_auth();

            if amount <= 0 {
                panic!("Amount must be positive");
            }

            let token_id: Address = env
                .storage()
                .instance()
                .get(&DataKey::Token)
                .expect("Not initialized");

            let spender_key = DataKey::Spender(caller.clone());
            let is_authorized: bool = env
                .storage()
                .persistent()
                .get(&spender_key)
                .unwrap_or(false);

            if !is_authorized {
                panic!("Caller is not an authorized spender");
            }
            // Bump the spender entry on every authorized read to keep it live
            bump_persistent(&env, &spender_key);

            let token_client = token::Client::new(&env, &token_id);
            token_client.transfer(&env.current_contract_address(), &learner, &amount);

            RewardDistributed {
                caller,
                learner,
                amount,
            }
            .publish(&env);
        }

        /// Funds the reward pool with tokens from a donor.
        ///
        /// # Panics
        /// * If contract is not initialized
        /// * If donor authentication fails
        pub fn fund_pool(env: Env, donor: Address, amount: i128) {
            donor.require_auth();

            let token_id: Address = env
                .storage()
                .instance()
                .get(&DataKey::Token)
                .expect("Not initialized");

            let token_client = token::Client::new(&env, &token_id);
            token_client.transfer(&donor, env.current_contract_address(), &amount);

            PoolFunded { donor, amount }.publish(&env);
        }

        /// Emergency sweep function allowing admin to transfer all tokens to a recovery wallet.
        ///
        /// # Panics
        /// * If contract is not initialized
        /// * If admin does not match stored admin
        pub fn emergency_sweep(env: Env, admin: Address, recovery_wallet: Address) {
            admin.require_auth();

            let stored_admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .expect("Not initialized");

            if admin != stored_admin {
                panic!("Unauthorized");
            }

            let token_id: Address = env
                .storage()
                .instance()
                .get(&DataKey::Token)
                .expect("Not initialized");

            let token_client = token::Client::new(&env, &token_id);
            let balance = token_client.balance(&env.current_contract_address());
            token_client.transfer(&env.current_contract_address(), &recovery_wallet, &balance);

            EmergencySweep {
                admin,
                recovery_wallet,
                amount: balance,
            }
            .publish(&env);
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

            env.deployer()
                .update_current_contract_wasm(new_wasm_hash.clone());

            ContractUpgraded {
                admin,
                new_wasm_hash,
            }
            .publish(&env);
        }
    }
}

#[cfg(feature = "contract")]
pub use contract_impl::RewardPool;

mod test;
