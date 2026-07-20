//! # Reward Pool Contract
//!
//! Central USDC reward distribution hub for the Orivex protocol.
//! Holds the reward-token balance and gates all payouts behind an
//! approved-spender allowlist, so only whitelisted contracts
//! (e.g. `CourseRegistry`, `QuestEngine`) can call
//! `distribute_reward`.
//!
//! ## Operational notes
//!
//! * The [`IsPaused`](types::DataKey::IsPaused) flag is a global
//!   circuit-breaker for `distribute_reward`.
//! * Fund recovery always routes through `emergency_sweep`; never
//!   via direct token transfer.
//! * Spender-list entries live in **persistent** storage and survive
//!   contract upgrades.
//!
//! ## Storage layout
//!
//! | Key | Tier | Type | Description |
//! |-----|------|------|-------------|
//! | `DataKey::Admin` | Instance | `Address` | Protocol admin |
//! | `DataKey::Token` | Instance | `Address` | SAC reward token |
//! | `DataKey::Spender(addr)` | Persistent | `bool` | Whitelist flag |
//! | `DataKey::IsPaused` | Instance | `bool` | Pause switch |

#![no_std]

/// Number of decimal places used by the reward token (Stellar standard).
pub const REWARD_TOKEN_DECIMALS: u32 = 7;

/// Maximum number of addresses that can be whitelisted as approved spenders.
pub const MAX_SPENDERS: u32 = 256;

/// Minimum valid payout amount (inclusive). Calls with `amount <= 0` panic.
pub const MIN_PAYOUT_AMOUNT: i128 = 1;

/// Platform fee expressed in basis points (1500 bp = 15 %).
/// Applied by callers (e.g. `QuestEngine`) before invoking `distribute_reward`.
pub const PLATFORM_FEE_BASIS_POINTS: u32 = 1500;
use soroban_sdk::{contractclient, contractevent, Address, BytesN, Env};

pub mod types;

/// Public interface for the RewardPool contract.
///
/// `#[contractclient]` generates `RewardPoolClient` from this trait so that
/// other contracts (e.g. `CourseRegistry`, `QuestEngine`) can trigger payouts
/// via cross-contract calls without importing the concrete implementation.
#[contractclient(name = "RewardPoolClient")]
pub trait RewardPoolInterface {
    /// See [`contract_impl::RewardPool::initialize`].
    fn initialize(env: Env, admin: Address, token: Address);
    /// See [`contract_impl::RewardPool::add_approved_spender`].
    fn add_approved_spender(env: Env, admin: Address, spender: Address);
    /// See [`contract_impl::RewardPool::set_pause`].
    fn set_pause(env: Env, admin: Address, status: bool);
    /// See [`contract_impl::RewardPool::distribute_reward`].
    fn distribute_reward(env: Env, caller: Address, learner: Address, amount: i128);
    /// See [`contract_impl::RewardPool::fund_pool`].
    fn fund_pool(env: Env, donor: Address, amount: i128);
    /// See [`contract_impl::RewardPool::emergency_sweep`].
    fn emergency_sweep(env: Env, admin: Address, recovery_wallet: Address);
    /// See [`contract_impl::RewardPool::upgrade_contract`].
    fn upgrade_contract(env: Env, admin: Address, new_wasm_hash: BytesN<32>);
}

/// Emitted once when the contract is successfully initialized.
#[contractevent]
pub struct PoolInitialized {
    /// Protocol admin address recorded at initialization.
    #[topic]
    pub admin: Address,
    /// SAC reward-token address recorded at initialization.
    #[topic]
    pub token: Address,
}

/// Emitted when a new address is added to the approved-spender whitelist.
#[contractevent]
pub struct SpenderAdded {
    /// The newly whitelisted spender address.
    #[topic]
    pub spender: Address,
}

/// Emitted on every successful `distribute_reward` call.
#[contractevent]
pub struct RewardDistributed {
    /// Whitelisted spender contract that triggered the payout.
    #[topic]
    pub caller: Address,
    /// Learner address that received the tokens.
    #[topic]
    pub learner: Address,
    /// Token amount transferred (in reward-token decimals).
    pub amount: i128,
}

/// Emitted when a donor tops up the pool's token balance.
#[contractevent]
pub struct PoolFunded {
    /// Donor address that sent the tokens.
    #[topic]
    pub donor: Address,
    /// Amount donated (in reward-token decimals).
    pub amount: i128,
}

/// Emitted when the admin drains the entire pool balance to a recovery wallet.
#[contractevent]
pub struct EmergencySweep {
    /// Admin who authorized the sweep.
    #[topic]
    pub admin: Address,
    /// Wallet that received the swept tokens.
    #[topic]
    pub recovery_wallet: Address,
    /// Full token balance that was swept.
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

#[cfg(feature = "contract")]
mod contract_impl {
    use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env};

    use crate::types::DataKey;
    use crate::{
        ContractUpgraded, EmergencySweep, PoolFunded, PoolInitialized, RewardDistributed,
        SpenderAdded,
    };

    #[contract]
    pub struct RewardPool;

    #[contractimpl]
    impl RewardPool {
        /// Initializes the RewardPool contract with the admin and reward-token addresses.
        ///
        /// Stores both addresses in instance storage and emits [`PoolInitialized`].
        /// Must be called exactly once after deployment; subsequent calls panic.
        ///
        /// # Arguments
        ///
        /// * `admin` — The protocol admin address (required auth).
        /// * `token` — The SAC reward-token address.
        ///
        /// # Panics
        ///
        /// * `"Already initialized"` — if `DataKey::Admin` already exists.
        ///
        /// # Examples
        ///
        /// ```rust,ignore
        /// client.initialize(&admin, &token);
        /// // second call panics with "Already initialized"
        /// ```
        pub fn initialize(env: Env, admin: Address, token: Address) {
            if env.storage().instance().has(&DataKey::Admin) {
                panic!("Already initialized");
            }
            admin.require_auth();
            env.storage().instance().set(&DataKey::Admin, &admin);
            env.storage().instance().set(&DataKey::Token, &token);
            PoolInitialized { admin, token }.publish(&env);
        }

        /// Adds a contract address to the approved-spender whitelist.
        ///
        /// Whitelisted addresses are permitted to call `distribute_reward`.
        /// The entry is recorded under `DataKey::Spender(spender)` in persistent
        /// storage, so it survives contract upgrades. Re-whitelisting is idempotent.
        ///
        /// # Arguments
        ///
        /// * `admin` — Must equal the stored admin (required auth).
        /// * `spender` — Contract address to whitelist.
        ///
        /// # Panics
        ///
        /// * `"Not initialized"` — if `DataKey::Admin` is absent.
        /// * `"Unauthorized"` — if `admin ≠ stored admin`.
        ///
        /// # Examples
        ///
        /// ```rust,ignore
        /// client.add_approved_spender(&admin, &course_registry_address);
        /// ```
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

            env.storage()
                .persistent()
                .set(&DataKey::Spender(spender.clone()), &true);

            SpenderAdded { spender }.publish(&env);
        }

        /// Toggles the global pause state of the contract.
        ///
        /// When paused (`status = true`), all `distribute_reward` calls panic
        /// with `"Contract is paused"`. Admin-only circuit-breaker for incidents.
        ///
        /// # Arguments
        ///
        /// * `admin` — Must equal the stored admin (required auth).
        /// * `status` — `true` to pause, `false` to unpause.
        ///
        /// # Panics
        ///
        /// * `"Not initialized"` — if `DataKey::Admin` is absent.
        /// * `"Unauthorized"` — if `admin ≠ stored admin`.
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

        /// Distributes reward tokens from the pool to a learner.
        ///
        /// The canonical payout path used by `CourseRegistry` and `QuestEngine`.
        /// The caller must be whitelisted via `add_approved_spender`, the amount
        /// must be strictly positive, and the contract must be unpaused.
        /// Tokens are transferred from this contract's own balance.
        ///
        /// # Arguments
        ///
        /// * `caller` — Whitelisted spender contract (required auth).
        /// * `learner` — Recipient of the reward tokens.
        /// * `amount` — Number of tokens to transfer (must be `> 0`).
        ///
        /// # Panics
        ///
        /// * `"Contract is paused"` — if `IsPaused` is `true`.
        /// * `"Amount must be positive"` — if `amount <= 0`.
        /// * `"Not initialized"` — if the contract hasn't been initialized.
        /// * `"Caller is not an authorized spender"` — if `caller` is not whitelisted.
        ///
        /// # Examples
        ///
        /// ```rust,ignore
        /// // registry must be whitelisted first
        /// client.add_approved_spender(&admin, &registry_addr);
        /// client.distribute_reward(&registry_addr, &learner, &100_0000000i128);
        /// ```
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

            let is_authorized: bool = env
                .storage()
                .persistent()
                .get(&DataKey::Spender(caller.clone()))
                .unwrap_or(false);

            if !is_authorized {
                panic!("Caller is not an authorized spender");
            }

            let token_client = token::Client::new(&env, &token_id);
            token_client.transfer(&env.current_contract_address(), &learner, &amount);

            RewardDistributed {
                caller,
                learner,
                amount,
            }
            .publish(&env);
        }

        /// Tops up the reward pool by transferring tokens from a donor.
        ///
        /// The donor must authorize the token transfer. On success the contract's
        /// token balance increases and [`PoolFunded`] is emitted.
        ///
        /// # Arguments
        ///
        /// * `donor` — Address supplying the tokens (required auth).
        /// * `amount` — Number of tokens to deposit.
        ///
        /// # Panics
        ///
        /// * `"Not initialized"` — if the contract hasn't been initialized.
        ///
        /// # Examples
        ///
        /// ```rust,ignore
        /// client.fund_pool(&donor, &500_0000000i128);
        /// ```
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

        /// Transfers the entire token balance to a recovery wallet. Admin-only.
        ///
        /// Intended for incidents requiring a full token rescue. Emits
        /// [`EmergencySweep`] with the swept amount.
        ///
        /// # Arguments
        ///
        /// * `admin` — Must equal the stored admin (required auth).
        /// * `recovery_wallet` — Destination address for the swept tokens.
        ///
        /// # Panics
        ///
        /// * `"Not initialized"` — if the contract hasn't been initialized.
        /// * `"Unauthorized"` — if `admin ≠ stored admin`.
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

        /// Upgrades the contract WASM to a new hash. Only callable by the protocol admin.
        ///
        /// Replaces the RewardPool WASM on the Soroban host and emits
        /// [`ContractUpgraded`] on success.
        ///
        /// # Arguments
        ///
        /// * `admin` — Must equal the stored admin (required auth).
        /// * `new_wasm_hash` — SHA-256 hash of the replacement WASM blob.
        ///
        /// # Panics
        ///
        /// * `"Not initialized"` — if the contract hasn't been initialized.
        /// * `"Unauthorized"` — if `admin ≠ stored admin`.
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
