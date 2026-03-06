#![no_std]
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, Symbol};

pub mod types;
use types::{Course, DataKey};

#[contract]
pub struct CourseRegistry;

#[contractimpl]
impl CourseRegistry {
    /// Registers a new course on-chain.
    ///
    /// # Arguments
    /// * `env`            - The Soroban environment
    /// * `admin`          - Protocol admin address (must authorise)
    /// * `instructor`     - Address of the course instructor
    /// * `total_modules`  - Number of modules in the course (must be > 0)
    /// * `metadata_hash`  - 32-byte hash of off-chain course metadata
    ///
    /// # Returns
    /// The newly assigned course ID (1-based, monotonically incrementing).
    pub fn create_course(
        env: Env,
        admin: Address,
        instructor: Address,
        total_modules: u32,
        metadata_hash: BytesN<32>,
    ) -> u32 {
        // 1. Authenticate the admin.
        admin.require_auth();

        // 2. Validate inputs.
        assert!(total_modules > 0, "total_modules must be greater than 0");

        // 3. Fetch and increment the global course counter (Instance storage).
        let current_count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::CourseCount)
            .unwrap_or(0);
        let new_id = current_count + 1;
        env.storage().instance().set(&DataKey::CourseCount, &new_id);

        // 4. Build the Course struct.
        let course = Course {
            instructor: instructor.clone(),
            total_modules,
            metadata_hash,
            active: true,
        };

        // 5. Persist to Persistent storage (survives ledger entry expiry extensions).
        env.storage()
            .persistent()
            .set(&DataKey::Course(new_id), &course);

        // 6. Emit a structured CourseCreated event.
        env.events().publish(
            (Symbol::new(&env, "CourseCreated"), new_id),
            (instructor, total_modules, new_id),
        );

        new_id
    }

    /// Returns the Course struct for the given course ID.
    ///
    /// Panics if the course does not exist.
    pub fn get_course(env: Env, course_id: u32) -> Course {
        env.storage()
            .persistent()
            .get(&DataKey::Course(course_id))
            .expect("course not found")
    }

    /// Returns the total number of courses registered so far.
    pub fn course_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::CourseCount)
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod test;
