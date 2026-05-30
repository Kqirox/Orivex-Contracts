#![no_std]

// Imports used unconditionally
use soroban_sdk::{contractevent, Address, Env, Vec};

// Imports only needed when building the standalone contract (wasm export shims)
#[cfg(feature = "contract")]
use soroban_sdk::{contract, contractimpl};

// When used as a library dependency, generate BadgeNFTClient via a trait so
// callers can make cross-contract calls without pulling in the export shims.
#[cfg(not(feature = "contract"))]
use soroban_sdk::contractclient;

pub mod types;
use types::Badge;

// DataKey is only referenced inside the contract impl block
#[cfg(feature = "contract")]
use types::DataKey;

// ── Client (available to all dependents) ─────────────────────────────────────

// When the `contract` feature is active, #[contractimpl] generates
// BadgeNFTClient automatically. When it is not active (i.e. when badge-nft is
// used as a library dependency), we generate it from this trait instead.
#[cfg(not(feature = "contract"))]
#[contractclient(name = "BadgeNFTClient")]
pub trait BadgeNFTInterface {
    fn initialize(env: Env, admin: Address);
    fn mint_badge(env: Env, caller: Address, learner: Address, course_id: u32);
    fn get_badges(env: Env, learner: Address) -> Vec<Badge>;
    fn get_badge_count(env: Env, learner: Address) -> u32;
    fn has_badge(env: Env, learner: Address, course_id: u32) -> bool;
}

// ── Event ─────────────────────────────────────────────────────────────────────

#[contractevent]
pub struct BadgeMinted {
    #[topic]
    pub learner: Address,
    #[topic]
    pub course_id: u32,
    pub minted_at: u64,
}

// ── Contract (standalone wasm only) ──────────────────────────────────────────

#[cfg(feature = "contract")]
#[contract]
pub struct BadgeNFT;

#[cfg(feature = "contract")]
#[contractimpl]
impl BadgeNFT {
    /// Initializes the BadgeNFT contract with the authorized registry address.
    /// Must be called once upon deployment.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Mints a Soulbound Token (badge) directly to the learner's address.
    /// Only the official protocol registry can trigger this.
    pub fn mint_badge(env: Env, caller: Address, learner: Address, course_id: u32) {
        caller.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        assert!(
            caller == stored_admin,
            "Unauthorized: Caller is not the authorized registry"
        );

        let badges_key = DataKey::UserBadges(learner.clone());

        let mut badges: Vec<Badge> = env
            .storage()
            .persistent()
            .get(&badges_key)
            .unwrap_or_else(|| Vec::new(&env));

        for existing_badge in badges.iter() {
            if existing_badge.course_id == course_id {
                panic!("Badge for this course already exists");
            }
        }

        let minted_at = env.ledger().timestamp();
        let new_badge = Badge {
            course_id,
            minted_at,
        };

        badges.push_back(new_badge);
        env.storage().persistent().set(&badges_key, &badges);

        BadgeMinted {
            learner,
            course_id,
            minted_at,
        }
        .publish(&env);
    }

    /// Returns all badges for a specific learner.
    pub fn get_badges(env: Env, learner: Address) -> Vec<Badge> {
        let badges_key = DataKey::UserBadges(learner);
        env.storage()
            .persistent()
            .get(&badges_key)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Returns the count of badges for a specific learner.
    pub fn get_badge_count(env: Env, learner: Address) -> u32 {
        let badges = Self::get_badges(env, learner);
        badges.len()
    }

    /// Checks if a learner has a specific badge.
    pub fn has_badge(env: Env, learner: Address, course_id: u32) -> bool {
        let badges = Self::get_badges(env, learner);
        for badge in badges.iter() {
            if badge.course_id == course_id {
                return true;
            }
        }
        false
    }
}

mod test;
