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

#[contractclient(name = "RewardPoolClient")]
pub trait RewardPoolInterface {
    fn initialize(env: Env, admin: Address, token: Address);
    fn add_approved_spender(env: Env, admin: Address, spender: Address);
    fn set_pause(env: Env, admin: Address, status: bool);
    fn distribute_reward(env: Env, caller: Address, learner: Address, amount: i128);
    fn fund_pool(env: Env, donor: Address, amount: i128);
    fn emergency_sweep(env: Env, admin: Address, recovery_wallet: Address);
    fn upgrade_contract(env: Env, admin: Address, new_wasm_hash: BytesN<32>);
    // ── Two-step admin transfer (Issue #20) ──────────────────────────
    fn propose_new_admin(env: Env, current_admin: Address, proposed: Address);
    fn accept_admin_ownership(env: Env, acceptor: Address);
    fn cancel_admin_transfer(env: Env, caller: Address);
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
    use contracts_common::two_step::{
        PendingTransfer, TransferAccepted, TransferCancelled, TransferProposed,
    };
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
        /// Initializes the RewardPool contract with admin and token addresses.
        ///
        /// # Arguments
        /// * `admin` - The admin address that will have administrative control
        /// * `token` - The SAC token address to be used as reward token
        ///
        /// # Panics
        /// * If contract is already initialized
        /// * If admin authentication fails
        /// Stores admin and reward-token addresses in instance storage and
        /// emits the `PoolInitialized` event. Both addresses are recorded
        /// on the first call; subsequent calls panic with
        /// `"Already initialized"`.
        pub fn initialize(env: Env, admin: Address, token: Address) {
            // 1. Check if already initialized
            if env.storage().instance().has(&DataKey::Admin) {
                panic!("Already initialized");
            }

            // 2. Require admin authentication
            admin.require_auth();

            // 3. Store admin in Instance storage
            env.storage().instance().set(&DataKey::Admin, &admin);

            // 4. Store token in Instance storage
            env.storage().instance().set(&DataKey::Token, &token);

            // 5. Emit PoolInitialized event
            PoolInitialized { admin, token }.publish(&env);
        }

        /// Adds a contract address to the approved spender whitelist.
        ///
        /// # Arguments
        /// * `admin` - The admin address (must match stored admin)
        /// * `spender` - The contract address to whitelist
        ///
        /// # Panics
        /// * If contract is not initialized
        /// * If admin does not match stored admin
        /// * If admin authentication fails
        /// Whitelist a caller contract so future `distribute_reward`
        /// calls from that contract's address are authorised. The
        /// spender is recorded under `DataKey::Spender(address)` in
        /// persistent storage. Re-whitelisting is allowed (idempotent).
        pub fn add_approved_spender(env: Env, admin: Address, spender: Address) {
            // 1. Fetch 'Admin' address from Instance storage
            let stored_admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .expect("Not initialized");

            // 2. Assert admin == stored_admin
            if admin != stored_admin {
                panic!("Unauthorized");
            }

            // 3. admin.require_auth()
            admin.require_auth();

            // 4. Save `true` to Persistent storage using DataKey::Spender(spender.clone())
            env.storage()
                .persistent()
                .set(&DataKey::Spender(spender.clone()), &true);

            // 5. Emit SpenderAdded event
            SpenderAdded { spender }.publish(&env);
        }

        /// Toggles the pause state of the contract (emergency circuit breaker).
        ///
        /// # Arguments
        /// * `admin` - The admin address (must match stored admin)
        /// * `status` - The pause status (true = paused, false = unpaused)
        ///
        /// # Panics
        /// * If contract is not initialized
        /// * If admin does not match stored admin
        /// * If admin authentication fails
        /// Sets the `IsPaused` flag in instance storage as a circuit
        /// breaker. Admin-only. When `IsPaused` is true,
        /// `distribute_reward` returns early with `"Contract is paused"`.
        pub fn set_pause(env: Env, admin: Address, status: bool) {
            // 1. Fetch 'Admin' address from Instance storage
            let stored_admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .expect("Not initialized");

            // 2. Assert admin == stored_admin
            if admin != stored_admin {
                panic!("Unauthorized");
            }

            // 3. admin.require_auth()
            admin.require_auth();

            // 4. Store pause status in Instance storage
            env.storage().instance().set(&DataKey::IsPaused, &status);
        }

        /// Distributes rewards from the pool to a learner.
        ///
        /// # Arguments
        /// * `caller` - The spender contract address (must be whitelisted)
        /// * `learner` - The learner address to receive the reward
        /// * `amount` - The amount of tokens to transfer
        ///
        /// # Panics
        /// * If caller authentication fails
        /// * If amount is not positive
        /// * If caller is not an authorized spender
        /// * If contract is not initialized
        /// Performs the canonical USDC payout path used by CourseRegistry.
        /// Spender must be whitelisted via `add_approved_spender`. The
        /// amount must be strictly positive. The contract must be unpaused.
        /// Funds are transferred from this contract's balance.
        pub fn distribute_reward(env: Env, caller: Address, learner: Address, amount: i128) {
            // 0. Check if contract is paused
            let is_paused: bool = env
                .storage()
                .instance()
                .get(&DataKey::IsPaused)
                .unwrap_or(false);
            assert!(!is_paused, "Contract is paused");

            // 1. caller.require_auth()
            caller.require_auth();

            // 2. Assert amount > 0
            if amount <= 0 {
                panic!("Amount must be positive");
            }

            // 3. Check if contract is initialized first
            let token_id: Address = env
                .storage()
                .instance()
                .get(&DataKey::Token)
                .expect("Not initialized");

            // 4. Construct DataKey::Spender(caller.clone())
            // 5. Fetch the boolean from Persistent storage. Assert it is true
            let is_authorized: bool = env
                .storage()
                .persistent()
                .get(&DataKey::Spender(caller.clone()))
                .unwrap_or(false);

            if !is_authorized {
                panic!("Caller is not an authorized spender");
            }

            // 6. Initialize token::Client::new(&env, &token_id)
            let token_client = token::Client::new(&env, &token_id);

            // 7. Call token_client.transfer(&env.current_contract_address(), &learner, &amount)
            token_client.transfer(&env.current_contract_address(), &learner, &amount);

            // 8. Emit RewardDistributed event
            RewardDistributed {
                caller,
                learner,
                amount,
            }
            .publish(&env);
        }

        /// Funds the reward pool with tokens from a donor.
        ///
        /// # Arguments
        /// * `donor` - The address donating the tokens
        /// * `amount` - The amount of tokens to donate
        ///
        /// # Panics
        /// * If contract is not initialized
        /// * If donor authentication fails
        /// * If token transfer fails
        /// Donor-funded top-up of the reward pool's token balance. The donor
        /// must authorize the token transfer; on success a `PoolFunded`
        /// event is published and the contract's balance increases.
        pub fn fund_pool(env: Env, donor: Address, amount: i128) {
            // 1. donor.require_auth()
            donor.require_auth();

            // 2. Fetch 'Token_Address' from Instance storage
            let token_id: Address = env
                .storage()
                .instance()
                .get(&DataKey::Token)
                .expect("Not initialized");

            // 3. Initialize token::Client::new(&env, &Token_Address)
            let token_client = token::Client::new(&env, &token_id);

            // 4. Call token_client.transfer(&donor, &env.current_contract_address(), &amount)
            token_client.transfer(&donor, env.current_contract_address(), &amount);

            // 5. Emit PoolFunded event
            PoolFunded { donor, amount }.publish(&env);
        }

        /// Emergency sweep function allowing admin to transfer all tokens from the contract
        /// to a recovery wallet in case of a critical vulnerability.
        ///
        /// # Arguments
        /// * `admin` - The admin address (must match stored admin)
        /// * `recovery_wallet` - The address to receive the swept tokens
        ///
        /// # Panics
        /// * If contract is not initialized
        /// * If admin does not match stored admin
        /// * If admin authentication fails
        /// Transfers the entire token balance of the contract to a
        /// designated recovery wallet. Admin-only. Emits
        /// `EmergencySweep` with the swept amount. Intended for
        /// incidents requiring a full token rescue.
        pub fn emergency_sweep(env: Env, admin: Address, recovery_wallet: Address) {
            // 1. admin.require_auth()
            admin.require_auth();

            // 2. Fetch stored admin from Instance storage
            let stored_admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .expect("Not initialized");

            // 3. Assert admin == stored_admin
            if admin != stored_admin {
                panic!("Unauthorized");
            }

            // 4. Fetch token address from Instance storage
            let token_id: Address = env
                .storage()
                .instance()
                .get(&DataKey::Token)
                .expect("Not initialized");

            // 5. Initialize token client
            let token_client = token::Client::new(&env, &token_id);

            // 6. Fetch full contract token balance
            let balance = token_client.balance(&env.current_contract_address());

            // 7. Transfer full balance to recovery wallet
            token_client.transfer(&env.current_contract_address(), &recovery_wallet, &balance);

            // 8. Emit EmergencySweep event
            EmergencySweep {
                admin,
                recovery_wallet,
                amount: balance,
            }
            .publish(&env);
        }

        /// Upgrades the contract WASM. Only callable by the Protocol Admin.
        /// Replaces the RewardPool WASM with the supplied hash on the
        /// Soroban host. Admin-only. Emits `ContractUpgraded` on
        /// successful deployment of the new WASM.
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

        // ── Two-step admin transfer (Issue #20) ──────────────────────
        // The admin can transfer the admin role without a typo ever
        // locking out the contract permanently: a typo is recovered by
        // calling `cancel_admin_transfer` before the typo'd address
        // calls `accept_admin_ownership`. See contracts/common::two_step.

        /// Stage 1 — propose a new admin. Only the current admin may call.
        pub fn propose_new_admin(
            env: Env,
            current_admin: Address,
            proposed: Address,
        ) {
            current_admin.require_auth();
            let stored_admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .expect("Not initialized");
            assert!(
                current_admin == stored_admin,
                "Unauthorized: Caller is not the admin"
            );

            let proposed_at = env.ledger().timestamp();
            env.storage().persistent().set(
                &DataKey::PendingAdmin,
                &PendingTransfer {
                    proposed: proposed.clone(),
                    proposed_at,
                },
            );

            TransferProposed {
                current: current_admin,
                proposed,
                proposed_at,
            }
            .publish(&env);
        }

        /// Stage 2 — accept the admin role. Only the proposed address may call.
        pub fn accept_admin_ownership(env: Env, acceptor: Address) {
            acceptor.require_auth();

            let pending: PendingTransfer = env
                .storage()
                .persistent()
                .get(&DataKey::PendingAdmin)
                .expect("No pending admin transfer");

            assert!(
                acceptor == pending.proposed,
                "Unauthorized: Acceptor is not the proposed admin"
            );

            let new_admin = pending.proposed.clone();
            env.storage().instance().set(&DataKey::Admin, &new_admin);
            env.storage().persistent().remove(&DataKey::PendingAdmin);

            TransferAccepted { new_value: new_admin }.publish(&env);
        }

        /// Cancel a pending admin transfer. Callable by either the
        /// current admin or by the proposed address (so a typo can be
        /// recovered before — or after — the typo'd caller reaches this
        /// contract).
        pub fn cancel_admin_transfer(env: Env, caller: Address) {
            caller.require_auth();

            let pending: PendingTransfer = env
                .storage()
                .persistent()
                .get(&DataKey::PendingAdmin)
                .expect("No pending admin transfer");

            let stored_admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .expect("Not initialized");

            assert!(
                caller == pending.proposed || caller == stored_admin,
                "Unauthorized: only proposer or current admin can cancel"
            );

            env.storage().persistent().remove(&DataKey::PendingAdmin);

            TransferCancelled {
                cancelled_by: caller,
                was_proposed: pending.proposed,
            }
            .publish(&env);
        }
    }
}

#[cfg(feature = "contract")]
pub use contract_impl::RewardPool;

mod test;
