//! # Stake Vault Contract
//!
//! Token staking with a one-week lock window and a three-tier basis-points
//! multiplier consumed by `QuestEngine` at review time.
//!
//! ## Operational notes
//!
//! * Multiplier tiers: `100` (default, 1×), `120` (≥ 100 staked, 1.2×),
//!   `200` (≥ 500 staked, 2×).
//! * Re-staking **resets** the lock timestamp, restarting the one-week window.
//! * After `unstake` the storage slot is deleted; subsequent `unstake` calls
//!   panic with `"No stake found"`.
//!
//! ## Storage layout
//!
//! | Key | Tier | Type | Description |
//! |-----|------|------|-------------|
//! | `DataKey::Admin` | Instance | `Address` | Protocol admin |
//! | `DataKey::Token` | Instance | `Address` | Staking token |
//! | `DataKey::UserStake(addr)` | Persistent | [`StakeInfo`] | Per-user stake record |

#![no_std]

/// Basis-points multiplier for the high stake tier (≥ 500 tokens). Equals 2×.
pub const STAKE_TIER_HIGH_BPS: u32 = 200;

/// Basis-points multiplier for the low stake tier (≥ 100 tokens). Equals 1.2×.
pub const STAKE_TIER_LOW_BPS: u32 = 120;

/// Basis-points multiplier when the user has no qualifying stake. Equals 1×.
pub const STAKE_TIER_NONE_BPS: u32 = 100;

/// Minimum staked amount to reach the **high** multiplier tier.
pub const TIER_HIGH_STAKE_BOUND: i128 = 500;

/// Minimum staked amount to reach the **low** multiplier tier.
pub const TIER_LOW_STAKE_BOUND: i128 = 100;

/// Default stake lock period in seconds (7 days = 604 800 s).
pub const DEFAULT_LOCK_PERIOD_SECONDS: u64 = 604800;
use soroban_sdk::{contract, contractevent, contractimpl, token, Address, BytesN, Env};

pub mod types;
use types::{DataKey, StakeInfo};

#[contract]
pub struct StakeVault;

/// Emitted once when the contract is successfully initialized.
#[contractevent]
pub struct StakeVaultInitialized {
    /// Protocol admin address recorded at initialization.
    #[topic]
    pub admin: Address,
    /// Staking token address recorded at initialization.
    #[topic]
    pub token: Address,
}

/// Emitted each time a user successfully stakes tokens.
#[contractevent]
pub struct Staked {
    /// User who performed the stake.
    #[topic]
    pub user: Address,
    /// Additional amount staked in this call.
    pub amount: i128,
    /// New cumulative staked balance after this call.
    pub total_staked: i128,
    /// Ledger timestamp at which the lock window was (re-)started.
    pub lock_timestamp: u64,
}

/// Emitted when a user successfully unstakes their full balance.
#[contractevent]
pub struct Unstaked {
    /// User who performed the unstake.
    #[topic]
    pub user: Address,
    /// Full amount returned to the user.
    pub amount: i128,
}

/// Emitted when the contract WASM is upgraded.
#[contractevent]
pub struct ContractUpgraded {
    /// Admin who authorized the upgrade.
    #[topic]
    pub admin: Address,
    /// SHA-256 hash of the new WASM blob.
    pub new_wasm_hash: BytesN<32>,
}

#[contractimpl]
impl StakeVault {
    /// Initializes the StakeVault with the admin and staking token addresses.
    ///
    /// Stores both addresses in instance storage and emits [`StakeVaultInitialized`].
    /// Must be called exactly once after deployment; subsequent calls panic.
    ///
    /// # Arguments
    ///
    /// * `admin` — Protocol admin address (required auth).
    /// * `token` — Address of the token users will stake.
    ///
    /// # Panics
    ///
    /// * `"Already initialized"` — if `DataKey::Admin` already exists.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// client.initialize(&admin, &token);
    /// ```
    pub fn initialize(env: Env, admin: Address, token: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);

        StakeVaultInitialized { admin, token }.publish(&env);
    }

    /// Locks `amount` tokens for `user` and resets the one-week lock window.
    ///
    /// Multiple `stake` calls accumulate in `StakeInfo.amount`. The
    /// `lock_timestamp` is always reset to the current ledger time, restarting
    /// the full [`DEFAULT_LOCK_PERIOD_SECONDS`] window on every call.
    ///
    /// # Arguments
    ///
    /// * `user` — Address staking the tokens (required auth).
    /// * `amount` — Tokens to lock; must be `> 0`.
    ///
    /// # Panics
    ///
    /// * `"Amount must be positive"` — if `amount <= 0`.
    /// * `"Not initialized"` — if the contract has not been initialized.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// client.stake(&user, &200i128);
    /// assert_eq!(client.get_multiplier(&user), 120); // 200 >= TIER_LOW_STAKE_BOUND
    /// ```
    pub fn stake(env: Env, user: Address, amount: i128) {
        user.require_auth();

        if amount <= 0 {
            panic!("Amount must be positive");
        }

        let token_id: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("Not initialized");
        let token_client = token::Client::new(&env, &token_id);

        token_client.transfer(&user, env.current_contract_address(), &amount);

        let now = env.ledger().timestamp();

        let mut stake_info: StakeInfo = env
            .storage()
            .persistent()
            .get(&DataKey::UserStake(user.clone()))
            .unwrap_or(StakeInfo {
                amount: 0,
                lock_timestamp: now,
            });

        stake_info.amount += amount;
        stake_info.lock_timestamp = now;

        env.storage()
            .persistent()
            .set(&DataKey::UserStake(user.clone()), &stake_info);

        Staked {
            user,
            amount,
            total_staked: stake_info.amount,
            lock_timestamp: stake_info.lock_timestamp,
        }
        .publish(&env);
    }

    /// Releases the caller's full staked balance once the lock period has elapsed.
    ///
    /// After a successful withdrawal the `DataKey::UserStake(user)` slot is
    /// deleted from persistent storage. A subsequent `unstake` call will panic
    /// with `"No stake found"`.
    ///
    /// # Arguments
    ///
    /// * `user` — Address withdrawing their stake (required auth).
    ///
    /// # Panics
    ///
    /// * `"No stake found"` — if `user` has no active stake record.
    /// * `"Lock period active"` — if fewer than [`DEFAULT_LOCK_PERIOD_SECONDS`]
    ///   seconds have elapsed since `lock_timestamp`.
    /// * `"Not initialized"` — if the contract has not been initialized.
    pub fn unstake(env: Env, user: Address) {
        user.require_auth();

        let stake_info: StakeInfo = env
            .storage()
            .persistent()
            .get(&DataKey::UserStake(user.clone()))
            .expect("No stake found");

        let lock_period: u64 = 604800;
        if env.ledger().timestamp() < stake_info.lock_timestamp + lock_period {
            panic!("Lock period active");
        }

        let token_id: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("Not initialized");
        let token_client = token::Client::new(&env, &token_id);

        token_client.transfer(
            &env.current_contract_address(),
            user.clone(),
            &stake_info.amount,
        );

        env.storage()
            .persistent()
            .remove(&DataKey::UserStake(user.clone()));

        Unstaked {
            user,
            amount: stake_info.amount,
        }
        .publish(&env);
    }

    /// Returns a basis-points multiplier based on the user's current staked balance.
    ///
    /// Tier table (basis points → effective multiplier):
    ///
    /// | Staked amount | BPS | Multiplier |
    /// |---------------|-----|-----------|
    /// | `< 100` | 100 | 1.0× |
    /// | `≥ 100` | 120 | 1.2× |
    /// | `≥ 500` | 200 | 2.0× |
    ///
    /// `QuestEngine` calls this at review time to scale learner payouts.
    /// Returns the `STAKE_TIER_NONE_BPS` (100) default if the user has no
    /// active stake.
    ///
    /// # Arguments
    ///
    /// * `user` — The address whose multiplier is being queried.
    ///
    /// # Returns
    ///
    /// A `u32` basis-points value: `100`, `120`, or `200`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(client.get_multiplier(&user), 100); // no stake
    /// client.stake(&user, &500i128);
    /// assert_eq!(client.get_multiplier(&user), 200); // high tier
    /// ```
    pub fn get_multiplier(env: Env, user: Address) -> u32 {
        let stake_info: StakeInfo = env
            .storage()
            .persistent()
            .get(&DataKey::UserStake(user))
            .unwrap_or(StakeInfo {
                amount: 0,
                lock_timestamp: 0,
            });

        if stake_info.amount >= 500 {
            200
        } else if stake_info.amount >= 100 {
            120
        } else {
            100
        }
    }

    /// Upgrades the contract WASM to a new hash. Only callable by the protocol admin.
    ///
    /// Replaces the StakeVault WASM on the Soroban host and emits
    /// [`ContractUpgraded`] on success.
    ///
    /// # Arguments
    ///
    /// * `admin` — Must equal the stored admin (required auth).
    /// * `new_wasm_hash` — SHA-256 hash of the replacement WASM blob.
    ///
    /// # Panics
    ///
    /// * `"Not initialized"` — if the contract has not been initialized.
    /// * `"Unauthorized"` — if `admin ≠ stored admin`.
    pub fn upgrade_contract(env: Env, admin: Address, new_wasm_hash: BytesN<32>) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }

        env.deployer()
            .update_current_contract_wasm(new_wasm_hash.clone());

        ContractUpgraded {
            admin,
            new_wasm_hash,
        }
        .publish(&env);
    }
}

mod test;
