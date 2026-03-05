#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Env, String};

/// Course registry contract for managing courses on the Orivex platform.
#[contract]
pub struct CourseRegistry;

/// Represents a course in the registry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Course {
    /// The instructor of the course
    pub instructor: String,
    /// The status of the course (e.g., "active", "draft", "archived")
    pub status: String,
    /// The number of modules in the course
    pub module_count: u32,
}

/// Data keys for persistent storage
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Course(u32),
}

#[contractimpl]
impl CourseRegistry {
    /// Returns the full details of a specific course.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `id` - The course ID
    ///
    /// # Returns
    /// The Course struct if found, or None if the course doesn't exist
    ///
    /// # Panics
    /// Panics if the course ID is invalid (course doesn't exist in storage)
    pub fn get_course(env: Env, id: u32) -> Course {
        // 1. Construct DataKey::Course(id)
        let key = DataKey::Course(id);
        
        // 2. Fetch Course struct from Persistent storage
        // 3. Assert course exists (panic if not found)
        env.storage()
            .persistent()
            .get(&key)
            .expect("Course not found")
    }

    /// Returns the full details of a specific course as an Option.
    ///
    /// # Arguments
    /// * `env` - The Soroban environment
    /// * `id` - The course ID
    ///
    /// # Returns
    /// Some(Course) if found, None if the course doesn't exist
    pub fn get_course_opt(env: Env, id: u32) -> Option<Course> {
        // 1. Construct DataKey::Course(id)
        let key = DataKey::Course(id);
        
        // 2. Fetch Course struct from Persistent storage
        // 3. Return Option<Course> (None if not found)
        env.storage()
            .persistent()
            .get(&key)
    }
}

#[cfg(test)]
mod test;
