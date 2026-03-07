#![no_std]
use soroban_sdk::{contract, contractevent, contractimpl, Address, BytesN, Env};

pub mod types;
use types::{Course, DataKey};

#[contract]
pub struct CourseRegistry;

#[contractevent]
pub struct MetadataUpdated {
    #[topic]
    pub id: u32,
    #[topic]
    pub instructor: Address,
    pub new_hash: BytesN<32>,
}

#[contractevent]
pub struct CourseCreated {
    #[topic]
    pub id: u32,
    #[topic]
    pub instructor: Address,
    pub total_modules: u32,
}

#[contractevent]
pub struct ModuleCompleted {
    #[topic]
    pub learner: Address,
    #[topic]
    pub course_id: u32,
    pub new_progress: u32,
}

#[contractimpl]
impl CourseRegistry {
    /// Sets the official Protocol Admin. Must be called once upon deployment.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    /// Registers a new course on-chain.
    pub fn create_course(
        env: Env,
        admin: Address,
        instructor: Address,
        total_modules: u32,
        metadata_hash: BytesN<32>,
    ) -> u32 {
        // Authenticate the caller's cryptographic signature.
        admin.require_auth();

        //  Verify the caller is the actual registered protocol admin.
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        assert!(
            admin == stored_admin,
            "Unauthorized: Caller is not the protocol admin"
        );

        // Validate inputs.
        assert!(total_modules > 0, "total_modules must be greater than 0");

        // Fetch and increment the global course counter.
        let current_count: u32 = env
            .storage()
            .instance()
            .get(&DataKey::CourseCount)
            .unwrap_or(0);
        let new_id = current_count + 1;
        env.storage().instance().set(&DataKey::CourseCount, &new_id);

        // Build and persist the Course struct.
        let course = Course {
            instructor: instructor.clone(),
            total_modules,
            metadata_hash,
            active: true,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Course(new_id), &course);

        // Emit the structured event using the V23 `.publish()` method.
        CourseCreated {
            id: new_id,
            instructor,
            total_modules,
        }
        .publish(&env);

        new_id
    }

    /// Updates the IPFS metadata hash for a course. Only callable by the course instructor.
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

    /// Helper to check the current total number of courses.
    pub fn course_count(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::CourseCount)
            .unwrap_or(0)
    }

    /// Records a learner's completion of a module after off-chain quiz validation.
    /// Only callable by the authorized verifier (protocol admin).
    pub fn complete_module(env: Env, verifier: Address, learner: Address, id: u32) {
        // 1. Authenticate the verifier's signature
        verifier.require_auth();

        // 2. Verify the verifier is the authorized protocol admin
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");
        assert!(
            verifier == stored_admin,
            "Unauthorized: Caller is not the protocol admin"
        );

        // 3. Retrieve the course to validate it exists and get total_modules
        let course: Course = env
            .storage()
            .persistent()
            .get(&DataKey::Course(id))
            .expect("Course not found");

        // 4. Retrieve current progress (defaults to 0 if not set)
        let current_progress: u32 = env
            .storage()
            .persistent()
            .get(&DataKey::Progress(learner.clone(), id))
            .unwrap_or(0);

        // 5. Assert current progress is less than total_modules
        assert!(
            current_progress < course.total_modules,
            "Course already completed"
        );

        // 6. Increment progress by 1
        let new_progress = current_progress + 1;

        // 7. Save new progress to persistent storage
        env.storage()
            .persistent()
            .set(&DataKey::Progress(learner.clone(), id), &new_progress);

        // 8. Emit ModuleCompleted event
        ModuleCompleted {
            learner,
            course_id: id,
            new_progress,
        }
        .publish(&env);
    }

    /// Returns the current progress of a learner for a specific course.
    pub fn get_progress(env: Env, learner: Address, id: u32) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::Progress(learner, id))
            .unwrap_or(0)
    }
}

mod test;
