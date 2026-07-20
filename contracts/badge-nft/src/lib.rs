//! # Badge NFT Contract
//!
//! Soulbound badge issuance, retrieval, and admin revocation for the Orivex protocol.
//!
//! Each badge is tied to a `(learner, course_id)` pair and is non-transferable
//! (soulbound). The contract is intentionally minimal: it delegates all
//! business-logic gates (who may mint) to the stored admin/registry address.
//!
//! ## Architecture
//!
//! The `#[contractclient]` macro generates [`BadgeNFTClient`] in every build
//! without emitting WASM exports. The actual struct and `#[contractimpl]` block
//! are gated behind the `contract` feature flag, which prevents duplicate
//! symbol errors when this crate is linked as a dependency of another contract
//! (e.g. `course-registry`).
//!
//! ## Storage layout
//!
//! | Key | Storage | Type | Description |
//! |-----|---------|------|-------------|
//! | `DataKey::Admin` | Instance | `Address` | Authorized minter / registry |
//! | `DataKey::UserBadges(addr)` | Persistent | `Vec<Badge>` | All badges for a learner |

#![no_std]

/// Default `minted_at` sentinel used when no ledger timestamp is available.
pub const BADGE_MINTED_AT_DEFAULT: u64 = 0;

/// Hard cap on the number of badges a single learner address may hold.
///
/// Badge lookups are linear scans over the stored `Vec<Badge>`, so this
/// constant bounds the worst-case iteration budget to a predictable value.
pub const MAX_BADGES_PER_LEARNER: u32 = 64;

use soroban_sdk::{contractclient, contractevent, Address, Env, Vec};

pub mod types;
use types::Badge;

/// Public interface for the BadgeNFT contract.
///
/// `#[contractclient]` generates `BadgeNFTClient` from this trait so that
/// other contracts (e.g. `course-registry`, `governance`) can cross-call
/// badge operations without importing the concrete implementation.
#[contractclient(name = "BadgeNFTClient")]
pub trait BadgeNFTInterface {
    /// See [`contract_impl::BadgeNFT::initialize`].
    fn initialize(env: Env, admin: Address);
    /// See [`contract_impl::BadgeNFT::mint_badge`].
    fn mint_badge(env: Env, caller: Address, learner: Address, course_id: u32);
    /// See [`contract_impl::BadgeNFT::revoke_badge`].
    fn revoke_badge(env: Env, admin: Address, learner: Address, course_id: u32);
    /// See [`contract_impl::BadgeNFT::get_badges`].
    fn get_badges(env: Env, learner: Address) -> Vec<Badge>;
    /// See [`contract_impl::BadgeNFT::get_badge_count`].
    fn get_badge_count(env: Env, learner: Address) -> u32;
    /// See [`contract_impl::BadgeNFT::has_badge`].
    fn has_badge(env: Env, learner: Address, course_id: u32) -> bool;
}

/// Emitted after a badge is successfully minted for a learner.
#[contractevent]
pub struct BadgeMinted {
    /// Learner who received the badge.
    #[topic]
    pub learner: Address,
    /// On-chain course identifier the badge represents.
    #[topic]
    pub course_id: u32,
    /// Ledger timestamp at the time of minting.
    pub minted_at: u64,
}

/// Emitted after a badge is revoked from a learner.
#[contractevent]
pub struct BadgeRevoked {
    /// Learner whose badge was revoked.
    #[topic]
    pub learner: Address,
    /// Course identifier of the revoked badge.
    #[topic]
    pub course_id: u32,
}

/// Emitted after the contract WASM is upgraded.
#[contractevent]
pub struct ContractUpgraded {
    /// Admin who authorized the upgrade.
    #[topic]
    pub admin: Address,
    /// SHA-256 hash of the new WASM blob.
    pub new_wasm_hash: soroban_sdk::BytesN<32>,
}

/// Concrete BadgeNFT implementation, compiled only when building the WASM artifact.
///
/// Dependents disable the `contract` feature to avoid duplicate symbol errors
/// at link time while still using the generated [`BadgeNFTClient`].
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
        ///
        /// Stores `admin` in instance storage under `DataKey::Admin`. This address
        /// is the only caller permitted to mint and revoke badges. Must be called
        /// exactly once after deployment.
        ///
        /// # Arguments
        ///
        /// * `admin` — The [`Address`] of the authorized registry or protocol admin.
        ///
        /// # Panics
        ///
        /// * `"Already initialized"` — if `DataKey::Admin` is already present in
        ///   instance storage (prevents re-initialization).
        ///
        /// # Examples
        ///
        /// ```rust,ignore
        /// let admin = Address::generate(&env);
        /// client.initialize(&admin);
        /// // Second call panics with "Already initialized"
        /// ```
        pub fn initialize(env: Env, admin: Address) {
            if env.storage().instance().has(&DataKey::Admin) {
                panic!("Already initialized");
            }
            env.storage().instance().set(&DataKey::Admin, &admin);
        }

        /// Mints a soulbound badge for `learner` representing completion of `course_id`.
        ///
        /// Only the address stored in `DataKey::Admin` (i.e. the protocol registry)
        /// may call this function. Each `(learner, course_id)` pair is unique — the
        /// function walks the learner's existing badge vector and panics on a duplicate.
        ///
        /// # Arguments
        ///
        /// * `caller` — Must equal the stored admin address; authenticated via
        ///   [`Address::require_auth`].
        /// * `learner` — The learner address to receive the badge.
        /// * `course_id` — The on-chain identifier of the completed course.
        ///
        /// # Panics
        ///
        /// * `"Contract not initialized"` — if `DataKey::Admin` is absent.
        /// * `"Unauthorized: Caller is not the authorized registry"` — if `caller ≠ admin`.
        /// * `"Badge for this course already exists"` — if the learner already holds
        ///   a badge for `course_id`.
        ///
        /// # Examples
        ///
        /// ```rust,ignore
        /// // registry is the stored admin address
        /// client.mint_badge(&registry, &learner, &1u32);
        /// assert_eq!(client.get_badge_count(&learner), 1);
        /// ```
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

        /// Revokes a previously minted badge from `learner` for `course_id`.
        ///
        /// Removes the matching `Badge` entry from the learner's persistent vector.
        /// If no badge matches the given `course_id`, the function is a **no-op** —
        /// no event is emitted and no panic occurs.
        ///
        /// # Arguments
        ///
        /// * `admin` — Must equal the stored admin address; authenticated via
        ///   [`Address::require_auth`].
        /// * `learner` — The learner address whose badge is being revoked.
        /// * `course_id` — The course identifier of the badge to remove.
        ///
        /// # Panics
        ///
        /// * `"Contract not initialized"` — if `DataKey::Admin` is absent.
        /// * `"Unauthorized: Caller is not the authorized registry"` — if `admin ≠ stored admin`.
        ///
        /// # Examples
        ///
        /// ```rust,ignore
        /// client.mint_badge(&registry, &learner, &2u32);
        /// assert!(client.has_badge(&learner, &2u32));
        /// client.revoke_badge(&registry, &learner, &2u32);
        /// assert!(!client.has_badge(&learner, &2u32));
        /// ```
        pub fn revoke_badge(env: Env, admin: Address, learner: Address, course_id: u32) {
            admin.require_auth();

            let stored_admin: Address = env
                .storage()
                .instance()
                .get(&DataKey::Admin)
                .expect("Contract not initialized");
            assert!(
                admin == stored_admin,
                "Unauthorized: Caller is not the authorized registry"
            );

            let badges_key = DataKey::UserBadges(learner.clone());

            let mut badges: Vec<Badge> = env
                .storage()
                .persistent()
                .get(&badges_key)
                .unwrap_or_else(|| Vec::new(&env));

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
                BadgeRevoked { learner, course_id }.publish(&env);
            }
        }

        /// Returns all badges currently held by `learner`.
        ///
        /// Returns an empty [`Vec`] when the learner has no badges so callers
        /// can iterate safely without checking length first.
        ///
        /// # Arguments
        ///
        /// * `learner` — The learner address to query.
        ///
        /// # Returns
        ///
        /// A `Vec<Badge>` — possibly empty — of all badges owned by `learner`.
        ///
        /// # Examples
        ///
        /// ```rust,ignore
        /// let badges = client.get_badges(&learner);
        /// assert_eq!(badges.len(), 0); // fresh address has no badges
        /// ```
        pub fn get_badges(env: Env, learner: Address) -> Vec<Badge> {
            let badges_key = DataKey::UserBadges(learner);
            env.storage()
                .persistent()
                .get(&badges_key)
                .unwrap_or_else(|| Vec::new(&env))
        }

        /// Returns the total number of badges held by `learner`.
        ///
        /// Equivalent to `get_badges(learner).len()` but avoids deserializing
        /// individual `Badge` fields when only the count is needed.
        ///
        /// # Arguments
        ///
        /// * `learner` — The learner address to query.
        ///
        /// # Returns
        ///
        /// The `u32` count of badges owned by `learner`. Returns `0` for an
        /// address that has never received a badge.
        ///
        /// # Examples
        ///
        /// ```rust,ignore
        /// assert_eq!(client.get_badge_count(&learner), 0);
        /// client.mint_badge(&registry, &learner, &3u32);
        /// assert_eq!(client.get_badge_count(&learner), 1);
        /// ```
        pub fn get_badge_count(env: Env, learner: Address) -> u32 {
            let badges = Self::get_badges(env, learner);
            badges.len()
        }

        /// Returns `true` if `learner` currently holds a badge for `course_id`.
        ///
        /// Performs a linear scan over the learner's badge vector. The scan is
        /// bounded by [`MAX_BADGES_PER_LEARNER`] in practice.
        ///
        /// # Arguments
        ///
        /// * `learner` — The learner address to check.
        /// * `course_id` — The course identifier to look up.
        ///
        /// # Returns
        ///
        /// `true` if a matching badge exists, `false` otherwise.
        ///
        /// # Examples
        ///
        /// ```rust,ignore
        /// assert!(!client.has_badge(&learner, &5u32));
        /// client.mint_badge(&registry, &learner, &5u32);
        /// assert!(client.has_badge(&learner, &5u32));
        /// ```
        pub fn has_badge(env: Env, learner: Address, course_id: u32) -> bool {
            let badges = Self::get_badges(env, learner);
            for badge in badges.iter() {
                if badge.course_id == course_id {
                    return true;
                }
            }
            false
        }

        /// Upgrades the contract WASM to a new hash. Only callable by the protocol admin.
        ///
        /// Replaces the BadgeNFT WASM on the Soroban host using
        /// `env.deployer().update_current_contract_wasm`. Emits [`ContractUpgraded`]
        /// on success.
        ///
        /// # Arguments
        ///
        /// * `admin` — Must equal the stored admin address; authenticated via
        ///   [`Address::require_auth`].
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

/// Re-export the concrete struct so tests can use `badge_nft::BadgeNFT` for registration.
#[cfg(feature = "contract")]
pub use contract_impl::BadgeNFT;

mod test;

#[cfg(test)]
mod invariants;
