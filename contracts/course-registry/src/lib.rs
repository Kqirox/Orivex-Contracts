//! # Course Registry Contract
//!
//! Manages the full lifecycle of on-chain courses for the Orivex protocol:
//! creation, metadata updates, enrollment, progress tracking, and completion.
//!
//! On final-module completion the registry performs two optional cross-contract
//! calls (when those addresses have been wired by the admin):
//!
//! 1. **BadgeNFT** — mints a soulbound badge for the learner.
//! 2. **RewardPool** — distributes a USDC payout to the learner.
//!
//! ## Architecture
//!
//! A single `Protocol Admin` address gates all privileged mutations (course
//! creation, status changes, and module verification). Instructors control
//! only their own course's metadata and ownership.
//!
//! ## Storage layout
//!
//! | Key | Tier | Type | Description |
//! |-----|------|------|-------------|
//! | `DataKey::Admin` | Instance | `Address` | Protocol admin |
//! | `DataKey::CourseCount` | Instance | `u32` | ID counter |
//! | `DataKey::BadgeNftAddress` | Instance | `Address` | Wired BadgeNFT |
//! | `DataKey::RewardPoolAddress` | Instance | `Address` | Wired RewardPool |
//! | `DataKey::Course(id)` | Persistent | [`Course`] | Course struct |
//! | `DataKey::Progress(addr, id)` | Persistent | `u32` | Modules completed |

#![no_std]

/// The first course ID allocated by [`CourseRegistry::create_course`].
///
/// Course IDs start at 1; 0 is reserved as a sentinel for "no course".
pub const INITIAL_COURSE_ID: u32 = 1;

/// Upper bound on the course ID space — the full `u32` range.
pub const MAX_COURSE_ID: u32 = u32::MAX;

/// Practical upper bound on `total_modules` per course.
///
/// Soroban storage cost is amortised over a single [`Course`] struct per ID,
/// but extremely large `total_modules` values would make the `Progress` counter
/// meaningless. This constant is advisory; enforcement is left to the admin.
pub const DEFAULT_TOTAL_MODULES_BOUND: u32 = 1000;

/// Base USDC reward amount distributed on course completion (7 decimal places).
///
/// Equals 10 USDC when the reward token uses 7 decimal places (Stellar standard).
pub const BASE_REWARD_AMOUNT: i128 = 10_0000000;

use soroban_sdk::{contract, contractevent, contractimpl, Address, BytesN, Env};

pub mod types;
use types::{Course, DataKey};

use badge_nft::BadgeNFTClient;
use reward_pool::RewardPoolClient;

#[contract]
pub struct CourseRegistry;

/// Emitted when a course instructor updates the IPFS metadata hash.
#[contractevent]
pub struct MetadataUpdated {
    /// ID of the course whose metadata was updated.
    #[topic]
    pub id: u32,
    /// Instructor who authorized the update.
    #[topic]
    pub instructor: Address,
    /// New 32-byte IPFS metadata hash.
    pub new_hash: BytesN<32>,
}

/// Emitted when a new course is registered on-chain.
#[contractevent]
pub struct CourseCreated {
    /// Auto-assigned ID for the new course.
    #[topic]
    pub id: u32,
    /// Instructor address associated with the course.
    #[topic]
    pub instructor: Address,
    /// Number of modules that must be completed.
    pub total_modules: u32,
}

/// Emitted when a course's active flag is toggled.
#[contractevent]
pub struct CourseStatusChanged {
    /// ID of the affected course.
    #[topic]
    pub id: u32,
    /// New active status (`true` = active, `false` = inactive).
    pub active: bool,
}

/// Emitted when a course is transferred to a new instructor.
#[contractevent]
pub struct OwnershipTransferred {
    /// ID of the transferred course.
    #[topic]
    pub course_id: u32,
    /// Previous instructor address.
    #[topic]
    pub previous_instructor: Address,
    /// New instructor address.
    pub new_instructor: Address,
}

/// Emitted each time a learner completes a module.
#[contractevent]
pub struct ModuleCompleted {
    /// Learner who completed the module.
    #[topic]
    pub learner: Address,
    /// Course in which the module was completed.
    #[topic]
    pub course_id: u32,
    /// Updated total module-completion count after this call.
    pub new_progress: u32,
}

/// Emitted when a learner completes the final module and receives a reward.
#[contractevent]
pub struct CourseCompleted {
    /// Learner who finished the course.
    #[topic]
    pub learner: Address,
    /// Completed course ID.
    #[topic]
    pub course_id: u32,
    /// USDC reward amount distributed (in token decimals).
    pub reward_amount: i128,
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
impl CourseRegistry {
    /// Initializes the CourseRegistry with the protocol admin address.
    ///
    /// Stores `admin` in instance storage under `DataKey::Admin`. This is the
    /// only call that does **not** require prior admin authentication — it is
    /// intended to be invoked exactly once by the deployer immediately after
    /// contract deployment.
    ///
    /// # Arguments
    ///
    /// * `admin` — Address to record as the protocol admin.
    ///
    /// # Panics
    ///
    /// * `"Already initialized"` — if `DataKey::Admin` is already set.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let admin = Address::generate(&env);
    /// client.initialize(&admin);
    /// // A second call panics with "Already initialized"
    /// ```
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Registers the RewardPool contract address for completion payouts.
    ///
    /// Wires the RewardPool so that `complete_module` can trigger a USDC payout
    /// when a learner finishes the final module. The RewardPool must separately
    /// whitelist the CourseRegistry via `add_approved_spender` before payouts
    /// will execute.
    ///
    /// # Arguments
    ///
    /// * `admin` — Must equal the stored protocol admin; authenticated via
    ///   [`Address::require_auth`].
    /// * `reward_pool_address` — Address of the deployed RewardPool contract.
    ///
    /// # Panics
    ///
    /// * `"Contract not initialized"` — if the registry has not been initialized.
    /// * `"Unauthorized: Caller is not the protocol admin"` — if `admin ≠ stored admin`.
    pub fn set_reward_pool_address(env: Env, admin: Address, reward_pool_address: Address) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        assert!(
            admin == stored_admin,
            "Unauthorized: Caller is not the protocol admin"
        );

        env.storage()
            .instance()
            .set(&DataKey::RewardPoolAddress, &reward_pool_address);
    }

    /// Registers the BadgeNFT contract address for completion badge minting.
    ///
    /// Wires the BadgeNFT so that `complete_module` can mint a soulbound badge
    /// when a learner finishes the final module.
    ///
    /// # Arguments
    ///
    /// * `admin` — Must equal the stored protocol admin; authenticated via
    ///   [`Address::require_auth`].
    /// * `badge_nft_address` — Address of the deployed BadgeNFT contract.
    ///
    /// # Panics
    ///
    /// * `"Contract not initialized"` — if the registry has not been initialized.
    /// * `"Unauthorized: Caller is not the protocol admin"` — if `admin ≠ stored admin`.
    pub fn set_badge_nft_address(env: Env, admin: Address, badge_nft_address: Address) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        assert!(
            admin == stored_admin,
            "Unauthorized: Caller is not the protocol admin"
        );

        env.storage()
            .instance()
            .set(&DataKey::BadgeNftAddress, &badge_nft_address);
    }

    /// Registers a new course on-chain and returns its auto-assigned ID.
    ///
    /// Allocates the next monotonically-increasing course ID, constructs a
    /// [`Course`] struct, and stores it in persistent storage under
    /// `DataKey::Course(id)`. Emits [`CourseCreated`].
    ///
    /// # Arguments
    ///
    /// * `admin` — Must equal the stored protocol admin; authenticated via
    ///   [`Address::require_auth`].
    /// * `instructor` — Address of the course instructor who will own metadata
    ///   updates and ownership transfers.
    /// * `total_modules` — Number of modules in the course; must be `> 0`.
    /// * `metadata_hash` — 32-byte IPFS CID hash for the course descriptor.
    ///
    /// # Returns
    ///
    /// The newly assigned `u32` course ID (starts at 1, increments by 1).
    ///
    /// # Panics
    ///
    /// * `"Contract not initialized"` — if the registry has not been initialized.
    /// * `"Unauthorized: Caller is not the protocol admin"` — if `admin ≠ stored admin`.
    /// * `"total_modules must be greater than 0"` — if `total_modules == 0`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let hash = BytesN::from_array(&env, &[0u8; 32]);
    /// let id = client.create_course(&admin, &instructor, &5u32, &hash);
    /// assert_eq!(id, 1u32); // first course gets ID 1
    /// ```
    pub fn create_course(
        env: Env,
        admin: Address,
        instructor: Address,
        total_modules: u32,
        metadata_hash: BytesN<32>,
    ) -> u32 {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        assert!(
            admin == stored_admin,
            "Unauthorized: Caller is not the protocol admin"
        );

        assert!(total_modules > 0, "total_modules must be greater than 0");

        let current_count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::CourseCount)
            .unwrap_or(0);
        let new_id = current_count + 1;
        env.storage().instance().set(&DataKey::CourseCount, &new_id);

        let course = Course {
            instructor: instructor.clone(),
            total_modules,
            metadata_hash,
            active: true,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Course(new_id), &course);

        CourseCreated {
            id: new_id,
            instructor,
            total_modules,
        }
        .publish(&env);

        new_id
    }

    /// Updates the IPFS metadata hash for a course.
    ///
    /// Only the current instructor of the course may call this function.
    /// Authentication is performed via `course.instructor.require_auth()`.
    ///
    /// # Arguments
    ///
    /// * `id` — ID of the course to update.
    /// * `new_hash` — Replacement 32-byte IPFS CID hash.
    ///
    /// # Panics
    ///
    /// * `"Course not found"` — if no course with `id` exists.
    /// * Soroban auth failure if the transaction signer is not the instructor.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let new_hash = BytesN::from_array(&env, &[1u8; 32]);
    /// client.update_metadata(&course_id, &new_hash);
    /// assert_eq!(client.get_course(&course_id).metadata_hash, new_hash);
    /// ```
    pub fn update_metadata(env: Env, id: u32, new_hash: BytesN<32>) {
        let mut course: Course = env
            .storage()
            .persistent()
            .get(&DataKey::Course(id))
            .expect("Course not found");

        course.instructor.require_auth();

        let instructor = course.instructor.clone();
        course.metadata_hash = new_hash.clone();

        env.storage()
            .persistent()
            .set(&DataKey::Course(id), &course);

        MetadataUpdated {
            id,
            instructor,
            new_hash,
        }
        .publish(&env);
    }

    /// Enrolls a learner in an active course, initializing their progress to zero.
    ///
    /// Writes `0u32` to `DataKey::Progress(learner, id)` in persistent storage.
    /// The learner must authorize the call. Enrolling in an inactive course or
    /// re-enrolling in a course the learner already joined both panic.
    ///
    /// # Arguments
    ///
    /// * `learner` — Address of the learner to enroll; authenticated via
    ///   [`Address::require_auth`].
    /// * `id` — ID of the course to enroll in.
    ///
    /// # Panics
    ///
    /// * `"Course not found"` — if no course with `id` exists.
    /// * `"Course is not active"` — if `course.active == false`.
    /// * `"Learner already enrolled"` — if a progress record already exists.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// client.enroll(&learner, &course_id);
    /// assert_eq!(client.get_progress(&learner, &course_id), 0);
    /// ```
    pub fn enroll(env: Env, learner: Address, id: u32) {
        learner.require_auth();

        let course: Course = env
            .storage()
            .persistent()
            .get(&DataKey::Course(id))
            .expect("Course not found");

        assert!(course.active, "Course is not active");

        let progress_key = DataKey::Progress(learner.clone(), id);
        assert!(
            !env.storage().persistent().has(&progress_key),
            "Learner already enrolled"
        );

        env.storage().persistent().set(&progress_key, &0u32);
    }

    /// Returns the total number of courses currently registered on-chain.
    ///
    /// Reads `DataKey::CourseCount` from instance storage. Returns `0` before
    /// the first `create_course` call.
    ///
    /// # Returns
    ///
    /// `u32` count of courses. The highest valid course ID equals this value
    /// (IDs are 1-based and contiguous).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(client.course_count(), 0);
    /// client.create_course(&admin, &instructor, &3u32, &hash);
    /// assert_eq!(client.course_count(), 1);
    /// ```
    pub fn course_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::CourseCount)
            .unwrap_or(0)
    }

    /// Toggles a course's active status. Only callable by the protocol admin.
    ///
    /// Persists the updated [`Course`] struct and emits [`CourseStatusChanged`].
    /// Deactivating a course preserves all existing learner progress records;
    /// it only prevents new enrollments.
    ///
    /// # Arguments
    ///
    /// * `admin` — Must equal the stored protocol admin; authenticated via
    ///   [`Address::require_auth`].
    /// * `id` — ID of the course to toggle.
    /// * `active` — New active status.
    ///
    /// # Panics
    ///
    /// * `"Contract not initialized"` — if the registry has not been initialized.
    /// * `"Unauthorized: Caller is not the protocol admin"` — if `admin ≠ stored admin`.
    /// * `"Course not found"` — if no course with `id` exists.
    pub fn set_course_status(env: Env, admin: Address, id: u32, active: bool) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        assert!(
            admin == stored_admin,
            "Unauthorized: Caller is not the protocol admin"
        );

        let mut course: Course = env
            .storage()
            .persistent()
            .get(&DataKey::Course(id))
            .expect("Course not found");

        course.active = active;
        env.storage()
            .persistent()
            .set(&DataKey::Course(id), &course);

        CourseStatusChanged { id, active }.publish(&env);
    }

    /// Returns `true` if the learner has completed all modules in the course.
    ///
    /// The check is intentionally defensive: progress values that exceed
    /// `total_modules` (which should never occur under normal operation) also
    /// return `true`.
    ///
    /// # Arguments
    ///
    /// * `learner` — The learner address to check.
    /// * `id` — The course ID to check against.
    ///
    /// # Returns
    ///
    /// `true` when `progress >= course.total_modules`, `false` otherwise.
    ///
    /// # Panics
    ///
    /// * `"Course not found"` — if no course with `id` exists.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert!(!client.is_course_finished(&learner, &course_id));
    /// // after all complete_module calls...
    /// assert!(client.is_course_finished(&learner, &course_id));
    /// ```
    pub fn is_course_finished(env: Env, learner: Address, id: u32) -> bool {
        let course: Course = env
            .storage()
            .persistent()
            .get(&DataKey::Course(id))
            .expect("Course not found");

        let progress: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Progress(learner, id))
            .unwrap_or(0);

        progress >= course.total_modules
    }

    /// Returns the full [`Course`] struct for the given ID.
    ///
    /// # Arguments
    ///
    /// * `id` — The course ID to look up.
    ///
    /// # Returns
    ///
    /// The [`Course`] struct stored under `DataKey::Course(id)`.
    ///
    /// # Panics
    ///
    /// * `"Course not found"` — if `id` has no corresponding record in storage.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let course = client.get_course(&1u32);
    /// assert_eq!(course.total_modules, 5u32);
    /// ```
    pub fn get_course(env: Env, id: u32) -> Course {
        env.storage()
            .persistent()
            .get(&DataKey::Course(id))
            .expect("Course not found")
    }

    /// Returns a learner's completed module count for a course.
    ///
    /// Returns `0` when the learner has not enrolled (i.e. no matching
    /// `DataKey::Progress` slot), so callers do not need to call `enroll`
    /// before reading progress.
    ///
    /// # Arguments
    ///
    /// * `learner` — The learner address to query.
    /// * `id` — The course ID.
    ///
    /// # Returns
    ///
    /// `u32` modules completed; `0` if the learner has no progress record.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// assert_eq!(client.get_progress(&learner, &course_id), 0);
    /// ```
    pub fn get_progress(env: Env, learner: Address, id: u32) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::Progress(learner, id))
            .unwrap_or(0)
    }

    /// Transfers ownership of a course to a new instructor address.
    ///
    /// Only the **current** instructor may call this. Authentication is
    /// verified by checking `course.instructor == current_instructor` before
    /// calling `current_instructor.require_auth()`. Emits [`OwnershipTransferred`].
    ///
    /// # Arguments
    ///
    /// * `current_instructor` — Must equal `course.instructor`; authenticated via
    ///   [`Address::require_auth`].
    /// * `new_instructor` — Address to hand ownership to.
    /// * `course_id` — ID of the course being transferred.
    ///
    /// # Panics
    ///
    /// * `"Course not found"` — if no course with `course_id` exists.
    /// * `"Unauthorized: Caller is not the course instructor"` — if
    ///   `current_instructor ≠ course.instructor`.
    pub fn transfer_ownership(
        env: Env,
        current_instructor: Address,
        new_instructor: Address,
        course_id: u32,
    ) {
        let mut course: Course = env
            .storage()
            .persistent()
            .get(&DataKey::Course(course_id))
            .expect("Course not found");

        assert!(
            course.instructor == current_instructor,
            "Unauthorized: Caller is not the course instructor"
        );

        current_instructor.require_auth();

        course.instructor = new_instructor.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Course(course_id), &course);

        OwnershipTransferred {
            course_id,
            previous_instructor: current_instructor,
            new_instructor,
        }
        .publish(&env);
    }

    /// Records a verifier-confirmed module completion for a learner.
    ///
    /// This is the core progression function. It increments the learner's
    /// module-completion counter and, on the **final** module, triggers two
    /// optional cross-contract calls (when addresses are wired):
    ///
    /// 1. `BadgeNFTClient::mint_badge` — mints a soulbound badge.
    /// 2. `RewardPoolClient::distribute_reward` — distributes [`BASE_REWARD_AMOUNT`]
    ///    USDC to the learner and emits [`CourseCompleted`].
    ///
    /// # Arguments
    ///
    /// * `verifier` — Must equal the stored protocol admin; authenticated via
    ///   [`Address::require_auth`].
    /// * `learner` — Address of the learner who completed the module.
    /// * `id` — Course ID in which the module was completed.
    ///
    /// # Panics
    ///
    /// * `"Contract not initialized"` — if the registry has not been initialized.
    /// * `"Unauthorized: Caller is not the protocol admin"` — if `verifier ≠ stored admin`.
    /// * `"Course not found"` — if no course with `id` exists.
    /// * `"Course already completed"` — if `progress >= total_modules` before this call.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Assuming a 1-module course and the learner is enrolled
    /// client.complete_module(&admin, &learner, &course_id);
    /// assert!(client.is_course_finished(&learner, &course_id));
    /// ```
    pub fn complete_module(env: Env, verifier: Address, learner: Address, id: u32) {
        verifier.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        assert!(
            verifier == stored_admin,
            "Unauthorized: Caller is not the protocol admin"
        );

        let course: Course = env
            .storage()
            .persistent()
            .get(&DataKey::Course(id))
            .expect("Course not found");

        let current_progress: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Progress(learner.clone(), id))
            .unwrap_or(0);

        assert!(
            current_progress < course.total_modules,
            "Course already completed"
        );

        let new_progress = current_progress + 1;

        env.storage()
            .persistent()
            .set(&DataKey::Progress(learner.clone(), id), &new_progress);

        ModuleCompleted {
            learner: learner.clone(),
            course_id: id,
            new_progress,
        }
        .publish(&env);

        // On the final module: mint badge and distribute reward (both optional).
        if new_progress == course.total_modules {
            if let Some(badge_nft_address) = env
                .storage()
                .instance()
                .get::<DataKey, Address>(&DataKey::BadgeNftAddress)
            {
                let badge_nft = BadgeNFTClient::new(&env, &badge_nft_address);
                badge_nft.mint_badge(&env.current_contract_address(), &learner, &id);
            }

            if let Some(reward_pool_address) = env
                .storage()
                .instance()
                .get::<DataKey, Address>(&DataKey::RewardPoolAddress)
            {
                let reward_pool = RewardPoolClient::new(&env, &reward_pool_address);
                let base_reward: i128 = 10_0000000; // 10 USDC (7 decimal places)
                reward_pool.distribute_reward(
                    &env.current_contract_address(),
                    &learner,
                    &base_reward,
                );

                CourseCompleted {
                    learner: learner.clone(),
                    course_id: id,
                    reward_amount: base_reward,
                }
                .publish(&env);
            }
        }
    }

    /// Upgrades the contract WASM to a new hash. Only callable by the protocol admin.
    ///
    /// Replaces the CourseRegistry WASM on the Soroban host. Emits
    /// [`ContractUpgraded`] on success.
    ///
    /// # Arguments
    ///
    /// * `admin` — Must equal the stored protocol admin; authenticated via
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

mod test;

#[cfg(test)]
mod invariants;
