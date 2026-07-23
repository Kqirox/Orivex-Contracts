#![no_std]

pub const STAKE_TIER_HIGH_BPS: u32 = 200;

pub const STAKE_TIER_LOW_BPS: u32 = 120;

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
use types::{DataKey, StakeInfo};

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

/// Re-exported two-step transfer events (Issue #20).
pub use contracts_common::two_step::{
    TransferAccepted, TransferCancelled, TransferProposed,
};

#[contractimpl]
impl StakeVault {
    // ── Two-step admin transfer (Issue #20) ──────────────────

    /// Stage 1 — propose a new admin. Only the current admin may call.
    pub fn propose_new_admin(
        env: Env,
        current_admin: Address,
        proposed: Address,
    ) {
        use contracts_common::two_step::PendingTransfer;

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
        use contracts_common::two_step::PendingTransfer;

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

    /// Cancel a pending admin transfer. Callable by the proposed
    /// address or the current admin.
    pub fn cancel_admin_transfer(env: Env, caller: Address) {
        use contracts_common::two_step::PendingTransfer;

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

    /// Returns a basis-points multiplier based on the user's staked
    /// amount. The scheme uses three tiers: 100 (default, 1.0x),
    /// 120 (≥100 stake, 1.2x), and 200 (≥500 stake, 2.0x). Quest
    /// review paths consult this value to scale payouts.
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
