#![no_std]

/// Basis-points value for the high-tier multiplier.
///
/// Prefer [`types::MultiplierBps::High`] and [`types::MultiplierBps::as_bps`]
/// over this bare constant.
#[deprecated(since = "0.2.0", note = "Use `MultiplierBps::High.as_bps()` instead")]
pub const STAKE_TIER_HIGH_BPS: u32 = 200;

/// Basis-points value for the low-tier multiplier.
///
/// Prefer [`types::MultiplierBps::Low`] and [`types::MultiplierBps::as_bps`]
/// over this bare constant.
#[deprecated(since = "0.2.0", note = "Use `MultiplierBps::Low.as_bps()` instead")]
pub const STAKE_TIER_LOW_BPS: u32 = 120;

/// Basis-points value for no multiplier (default tier).
///
/// Prefer [`types::MultiplierBps::None`] and [`types::MultiplierBps::as_bps`]
/// over this bare constant.
#[deprecated(since = "0.2.0", note = "Use `MultiplierBps::None.as_bps()` instead")]
pub const STAKE_TIER_NONE_BPS: u32 = 100;
// Operational notes — multiplier calculation is a 3-tier
// lookup bound by `TIER_LOW_STAKE_BOUND` and
// `TIER_HIGH_STAKE_BOUND`. Re-staking resets the lock
// timestamp. Lock period is one week by default
// (`DEFAULT_LOCK_PERIOD_SECONDS`).

pub const TIER_HIGH_STAKE_BOUND: i128 = 500;

pub const TIER_LOW_STAKE_BOUND: i128 = 100;

pub const DEFAULT_LOCK_PERIOD_SECONDS: u64 = 604800;
// Crate overview — stake lock holding and multiplier computation.
// Provides `get_multiplier(user)` for cross-contract use by
// QuestEngine on review-time payout calculation.
use soroban_sdk::{contract, contractevent, contractimpl, token, Address, BytesN, Env};

pub mod types;
use types::{DataKey, MultiplierBps, StakeInfo};

#[contract]
pub struct StakeVault;

#[contractevent]
pub struct StakeVaultInitialized {
    #[topic]
    pub admin: Address,
    #[topic]
    pub token: Address,
}

#[contractevent]
pub struct Staked {
    #[topic]
    pub user: Address,
    pub amount: i128,
    pub total_staked: i128,
    pub lock_timestamp: u64,
}

#[contractevent]
pub struct Unstaked {
    #[topic]
    pub user: Address,
    pub amount: i128,
}

#[contractevent]
pub struct ContractUpgraded {
    #[topic]
    pub admin: Address,
    pub new_wasm_hash: BytesN<32>,
}

#[contractimpl]
impl StakeVault {
    /// Initializes the StakeVault with admin and reward token
    /// addresses and emits `StakeVaultInitialized`. Admin-only at
    /// deploy time. Re-initialization panics with
    /// `"Already initialized"`.
    pub fn initialize(env: Env, admin: Address, token: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);

        StakeVaultInitialized { admin, token }.publish(&env);
    }

    /// Locks tokens for the configured lock period and resets the
    /// caller's `lock_timestamp` to the current ledger time. Multi-call
    /// stakes accumulate in the same `StakeInfo.amount` field.
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

    /// Releases the caller's full staked balance once the lock period
    /// has elapsed. After successful withdrawal the storage slot is
    /// cleared — subsequent `unstake` calls panic with `"No stake found"`.
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

    /// Returns the basis-points multiplier tier for the given user based on
    /// their current staked balance.
    ///
    /// ## Tier table
    ///
    /// | Staked amount | Variant              | BPS | Effective multiplier |
    /// |---------------|----------------------|-----|----------------------|
    /// | < 100         | [`MultiplierBps::None`]  | 100 | 1.0×             |
    /// | 100 – 499     | [`MultiplierBps::Low`]   | 120 | 1.2×             |
    /// | ≥ 500         | [`MultiplierBps::High`]  | 200 | 2.0×             |
    ///
    /// ## Basis-points convention
    ///
    /// All variants are expressed in *basis points* (hundredths of 1×).
    /// Cross-contract callers **must** call [`MultiplierBps::as_bps`] and then
    /// divide by 100 to obtain the scaled payout:
    ///
    /// ```ignore
    /// let multiplier = stake_vault.get_multiplier(&learner);
    /// let boosted = (base_amount * multiplier.as_bps() as i128) / 100;
    /// ```
    ///
    /// Using the raw discriminant integer instead of `as_bps()` will produce
    /// incorrect results (0, 1, or 2 rather than 100, 120, or 200).
    pub fn get_multiplier(env: Env, user: Address) -> MultiplierBps {
        let stake_info: StakeInfo = env
            .storage()
            .persistent()
            .get(&DataKey::UserStake(user))
            .unwrap_or(StakeInfo {
                amount: 0,
                lock_timestamp: 0,
            });

        if stake_info.amount >= TIER_HIGH_STAKE_BOUND {
            MultiplierBps::High
        } else if stake_info.amount >= TIER_LOW_STAKE_BOUND {
            MultiplierBps::Low
        } else {
            MultiplierBps::None
        }
    }

    /// Replaces the StakeVault WASM with the supplied hash on the
    /// Soroban host. Admin-only. Emits `ContractUpgraded` on
    /// successful deployment.
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
