#![no_std]
use soroban_sdk::{contractclient, contractevent, Address, Env, Vec};

pub mod types;
use types::Badge;

// `#[contractclient]` generates `BadgeNFTClient` in every build (no wasm exports).
// `#[contractimpl]` on the struct below generates the wasm exports, but only
// when the `contract` feature is enabled — preventing duplicate symbols when
// this crate is linked as a dependency of another contract.
#[contractclient(name = "BadgeNFTClient")]
pub trait BadgeNFTInterface {
    fn initialize(env: Env, admin: Address);
    fn mint_badge(env: Env, caller: Address, learner: Address, course_id: u32);
    fn get_badges(env: Env, learner: Address) -> Vec<Badge>;
    fn get_badge_count(env: Env, learner: Address) -> u32;
    fn has_badge(env: Env, learner: Address, course_id: u32) -> bool;
}

#[contractevent]
pub struct BadgeMinted {
    #[topic]
    pub learner: Address,
    #[topic]
    pub course_id: u32,
    pub minted_at: u64,
}

// The actual contract struct and implementation are only compiled when building
// the badge-nft wasm itself (default feature). Dependents disable this feature
// to avoid duplicate symbol errors at link time.
#[cfg(feature = "contract")]
mod contract_impl {
    use soroban_sdk::{contract, contractimpl, Address, Env, Vec};

    use crate::types::{Badge, DataKey};
    use crate::BadgeMinted;

    #[contract]
    pub struct BadgeNFT;

    #[contractimpl]
    impl BadgeNFT {
        /// Initializes the BadgeNFT contract with the authorized registry address.
        /// Must be called once upon deployment.
        ///
        /// # Panics
        /// * If contract is already initialized
        pub fn initialize(env: Env, admin: Address) {
            if env.storage().instance().has(&DataKey::Admin) {
                panic!("Already initialized");
            }
            env.storage().instance().set(&DataKey::Admin, &admin);
        }

        /// Mints a Soulbound Token (badge) directly to the learner's address.
        /// Only the official protocol registry can trigger this.
        ///
        /// # Panics
        /// * If caller authentication fails
        /// * If caller is not the authorized registry
        /// * If learner already has a badge for this course_id (duplicate minting)
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
}

// Re-export the struct so tests can use `badge_nft::BadgeNFT` for registration.
#[cfg(feature = "contract")]
pub use contract_impl::BadgeNFT;

mod test;
