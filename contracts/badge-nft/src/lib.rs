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
    fn revoke_badge(env: Env, admin: Address, learner: Address, course_id: u32);
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

#[contractevent]
pub struct BadgeRevoked {
    #[topic]
    pub learner: Address,
    #[topic]
    pub course_id: u32,
}

#[contractevent]
pub struct ContractUpgraded {
    #[topic]
    pub admin: Address,
    pub new_wasm_hash: soroban_sdk::BytesN<32>,
}

// The actual contract struct and implementation are only compiled when building
// the badge-nft wasm itself (default feature). Dependents disable this feature
// to avoid duplicate symbol errors at link time.
#[cfg(feature = "contract")]
mod contract_impl {
    use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Vec};

    use crate::types::{Badge, DataKey};
    use crate::{BadgeMinted, BadgeRevoked, ContractUpgraded};

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

        /// Revokes a Soulbound Token (badge) from a learner's address.
        /// Only the official protocol registry can trigger this for fraud prevention.
        ///
        /// # Arguments
        /// * `admin` - The caller address (must be the authorized registry)
        /// * `learner` - The learner address to revoke the badge from
        /// * `course_id` - The course ID of the badge to revoke
        ///
        /// # Panics
        /// * If caller authentication fails
        /// * If caller is not the authorized registry
        pub fn revoke_badge(env: Env, admin: Address, learner: Address, course_id: u32) {
            // 1. admin.require_auth()
            admin.require_auth();

            // 2. Fetch 'Admin' (Registry) address from Instance storage. Assert caller == Admin.
            let stored_admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .expect("Contract not initialized");
            assert!(
                admin == stored_admin,
                "Unauthorized: Caller is not the authorized registry"
            );

            // 3. Construct DataKey::UserBadges(learner).
            let badges_key = DataKey::UserBadges(learner.clone());

            // 4. Fetch existing Vec<Badge>.
            let mut badges: Vec<Badge> = env
                .storage()
                .persistent()
                .get(&badges_key)
                .unwrap_or_else(|| Vec::new(&env));

            // 5. Find the badge with course_id and remove it.
            let mut found = false;
            let mut index_to_remove = 0;
            for (i, badge) in badges.iter().enumerate() {
                if badge.course_id == course_id {
                    index_to_remove = i as u32;
                    found = true;
                    break;
                }
            }

            if found {
                badges.remove(index_to_remove);
                env.storage().persistent().set(&badges_key, &badges);

                // 6. Emit BadgeRevoked event.
                BadgeRevoked { learner, course_id }.publish(&env);
            }
        }

        /// Returns all badges for a specific learner.
        ///
        /// # Arguments
        /// * `learner` - The learner address
        ///
        /// # Returns
        /// Vector of Badge structs. Returns empty vector if learner has no badges.
        pub fn get_badges(env: Env, learner: Address) -> Vec<Badge> {
            let badges_key = DataKey::UserBadges(learner);
            env.storage()
                .persistent()
                .get(&badges_key)
                .unwrap_or_else(|| Vec::new(&env))
        }

        /// Returns the count of badges for a specific learner.
        ///
        /// # Arguments
        /// * `learner` - The learner address
        ///
        /// # Returns
        /// Number of badges the learner owns.
        pub fn get_badge_count(env: Env, learner: Address) -> u32 {
            let badges = Self::get_badges(env, learner);
            badges.len()
        }

        /// Checks if a learner has a specific badge.
        ///
        /// # Arguments
        /// * `learner` - The learner address
        /// * `course_id` - The course ID to check
        ///
        /// # Returns
        /// true if the learner has the badge, false otherwise.
        pub fn has_badge(env: Env, learner: Address, course_id: u32) -> bool {
            let badges = Self::get_badges(env, learner);
            for badge in badges.iter() {
                if badge.course_id == course_id {
                    return true;
                }
            }
            false
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

// Re-export the struct so tests can use `badge_nft::BadgeNFT` for registration.
#[cfg(feature = "contract")]
pub use contract_impl::BadgeNFT;

mod test;
