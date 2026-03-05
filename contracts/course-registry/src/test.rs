use soroban_sdk::{Env, String};

use crate::{Course, DataKey};

#[test]
fn test_get_course_exists() {
    let env = Env::default();
    let contract_id = env.register(crate::CourseRegistry, ());
    let client = crate::CourseRegistryClient::new(&env, &contract_id);

    // Setup: Store a course in persistent storage
    let course_id = 1u32;
    let course = Course {
        instructor: String::from_str(&env, "Dr. Jane Smith"),
        status: String::from_str(&env, "active"),
        module_count: 5,
    };

    let key = DataKey::Course(course_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&key, &course);
    });

    // Test: Retrieve the course
    let retrieved_course = client.get_course(&course_id);

    // Assert: Verify all fields match
    assert_eq!(retrieved_course.instructor, course.instructor);
    assert_eq!(retrieved_course.status, course.status);
    assert_eq!(retrieved_course.module_count, course.module_count);
    assert_eq!(retrieved_course, course);
}

#[test]
#[should_panic(expected = "Course not found")]
fn test_get_course_not_exists() {
    let env = Env::default();
    let contract_id = env.register(crate::CourseRegistry, ());
    let client = crate::CourseRegistryClient::new(&env, &contract_id);

    // Test: Try to retrieve a non-existent course
    let course_id = 999u32;
    let _ = client.get_course(&course_id);
}

#[test]
fn test_get_course_opt_exists() {
    let env = Env::default();
    let contract_id = env.register(crate::CourseRegistry, ());
    let client = crate::CourseRegistryClient::new(&env, &contract_id);

    // Setup: Store a course in persistent storage
    let course_id = 2u32;
    let course = Course {
        instructor: String::from_str(&env, "Prof. John Doe"),
        status: String::from_str(&env, "draft"),
        module_count: 3,
    };

    let key = DataKey::Course(course_id);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&key, &course);
    });

    // Test: Retrieve the course using get_course_opt
    let retrieved_course = client.get_course_opt(&course_id);

    // Assert: Verify Some(Course) is returned with correct data
    assert!(retrieved_course.is_some());
    let course_opt = retrieved_course.unwrap();
    assert_eq!(course_opt.instructor, course.instructor);
    assert_eq!(course_opt.status, course.status);
    assert_eq!(course_opt.module_count, course.module_count);
    assert_eq!(course_opt, course);
}

#[test]
fn test_get_course_opt_not_exists() {
    let env = Env::default();
    let contract_id = env.register(crate::CourseRegistry, ());
    let client = crate::CourseRegistryClient::new(&env, &contract_id);

    // Test: Try to retrieve a non-existent course using get_course_opt
    let course_id = 888u32;
    let retrieved_course = client.get_course_opt(&course_id);

    // Assert: Verify None is returned
    assert!(retrieved_course.is_none());
}

#[test]
fn test_get_course_multiple_courses() {
    let env = Env::default();
    let contract_id = env.register(crate::CourseRegistry, ());
    let client = crate::CourseRegistryClient::new(&env, &contract_id);

    // Setup: Store multiple courses
    let course1 = Course {
        instructor: String::from_str(&env, "Instructor A"),
        status: String::from_str(&env, "active"),
        module_count: 10,
    };
    let course2 = Course {
        instructor: String::from_str(&env, "Instructor B"),
        status: String::from_str(&env, "archived"),
        module_count: 7,
    };

    let key1 = DataKey::Course(1u32);
    let key2 = DataKey::Course(2u32);
    env.as_contract(&contract_id, || {
        env.storage().persistent().set(&key1, &course1);
        env.storage().persistent().set(&key2, &course2);
    });

    // Test: Retrieve both courses
    let retrieved_course1 = client.get_course(&1u32);
    let retrieved_course2 = client.get_course(&2u32);

    // Assert: Verify each course is retrieved correctly
    assert_eq!(retrieved_course1, course1);
    assert_eq!(retrieved_course2, course2);
    assert_ne!(retrieved_course1, retrieved_course2);
}
