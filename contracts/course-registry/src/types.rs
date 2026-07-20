//! Shared types for the CourseRegistry contract.

use soroban_sdk::{contracttype, Address, BytesN};

/// On-chain representation of a course registered in the protocol.
///
/// Stored in persistent storage under `DataKey::Course(id)`.
///
/// # Field notes
///
/// * `instructor` is the address that controls metadata updates and
///   ownership transfers for this course.
/// * `total_modules` must be `> 0` — enforced at creation time.
/// * `metadata_hash` is a 32-byte hash pointing at IPFS CID metadata
///   (title, description, syllabus, etc.).
/// * `active` gates enrollment; a deactivated course rejects new learners
///   but preserves all existing progress records.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Course {
    /// Address of the course instructor / owner.
    pub instructor: Address,
    /// Total number of modules that must be completed to finish the course.
    pub total_modules: u32,
    /// IPFS metadata hash for the course content descriptor.
    pub metadata_hash: BytesN<32>,
    /// Whether the course is accepting new enrollments.
    pub active: bool,
}

/// Storage keys used by the CourseRegistry contract.
///
/// | Variant | Storage tier | Type | Description |
/// |---------|-------------|------|-------------|
/// | `Course(u32)` | Persistent | [`Course`] | Course struct keyed by ID |
/// | `Progress(Address, u32)` | Persistent | `u32` | Modules completed by a learner |
/// | `CourseCount` | Instance | `u32` | Monotonically-increasing ID counter |
/// | `Admin` | Instance | `Address` | Protocol admin address |
/// | `BadgeNftAddress` | Instance | `Address` | Wired BadgeNFT contract |
/// | `RewardPoolAddress` | Instance | `Address` | Wired RewardPool contract |
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Persistent storage key for a [`Course`] identified by `u32` ID.
    Course(u32),
    /// Persistent storage key for a learner's module-completion count
    /// for a specific course: `(learner_address, course_id)`.
    Progress(Address, u32),
    /// Instance storage key tracking the highest allocated course ID.
    CourseCount,
    /// Instance storage key for the protocol admin [`Address`].
    Admin,
    /// Instance storage key for the BadgeNFT contract [`Address`].
    BadgeNftAddress,
    /// Instance storage key for the RewardPool contract [`Address`].
    RewardPoolAddress,
}
