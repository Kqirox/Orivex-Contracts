//! Shared types for the QuestEngine contract.

use soroban_sdk::{contracttype, Address, BytesN};

/// Discriminates between the two quest funding models.
///
/// | Variant | Funded by | Reviewed by | Payout source |
/// |---------|-----------|-------------|---------------|
/// | `Build` | Employer (locked on creation) | Employer | QuestEngine vault |
/// | `Explore` | RewardPool | Protocol admin | RewardPool via `distribute_reward` |
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuestType {
    /// Employer-funded bounty. Reward tokens are locked in the QuestEngine
    /// contract at creation time and released on submission approval.
    Build,
    /// Admin-verified off-chain action. Reward comes from the RewardPool;
    /// no tokens are locked in the QuestEngine itself.
    Explore,
}

/// An on-chain quest record, keyed by auto-incremented ID.
///
/// Stored in persistent storage under `DataKey::Quest(id)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Quest {
    /// Address of the employer (Build) or admin (Explore) who created the quest.
    pub employer: Address,
    /// Reward tokens allocated for this quest.
    /// * Build: locked in the contract at creation.
    /// * Explore: pulled from the RewardPool on verification.
    pub reward_amount: i128,
    /// Funding and review model for this quest.
    pub quest_type: QuestType,
    /// IPFS hash of the quest description, requirements, and evaluation rubric.
    pub metadata_hash: BytesN<32>,
    /// Whether the quest is still accepting submissions or verification.
    /// Set to `false` after `refund_quest` is called.
    pub active: bool,
}

/// Lifecycle states for a learner's proof submission.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubmissionStatus {
    /// Submitted and awaiting employer review.
    Pending,
    /// Approved by the employer; reward has been transferred.
    Approved,
    /// Rejected by the employer; no reward issued.
    Rejected,
}

/// A proof submission from a learner for a Build quest.
///
/// Stored in persistent storage under `DataKey::Submission(learner, quest_id)`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Submission {
    /// IPFS or on-chain hash of the learner's proof artifact.
    pub proof_hash: BytesN<32>,
    /// Current lifecycle state of this submission.
    pub status: SubmissionStatus,
}

/// Storage keys used by the QuestEngine contract.
///
/// | Variant | Storage tier | Type | Description |
/// |---------|-------------|------|-------------|
/// | `Admin` | Instance | `Address` | Protocol admin address |
/// | `Quest(u32)` | Persistent | [`Quest`] | Quest record by ID |
/// | `Submission(Address, u32)` | Persistent | [`Submission`] | Per-learner submission |
/// | `Token` | Instance | `Address` | USDC token address |
/// | `QuestCounter` | Instance | `u32` | Auto-increment quest ID |
/// | `RewardPool` | Instance | `Address` | Wired RewardPool contract |
/// | `IsPaused` | Instance | `bool` | Global pause circuit-breaker |
/// | `StakeVault` | Instance | `Address` | Wired StakeVault contract |
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Instance storage key for the protocol admin [`Address`].
    Admin,
    /// Persistent storage key for a [`Quest`] identified by `u32` ID.
    Quest(u32),
    /// Persistent storage key for a learner's [`Submission`] on a given quest.
    /// Tuple: `(submitter_address, quest_id)`.
    Submission(Address, u32),
    /// Instance storage key for the USDC token [`Address`].
    Token,
    /// Instance storage key for the current quest ID counter.
    QuestCounter,
    /// Instance storage key for the RewardPool contract [`Address`].
    RewardPool,
    /// Instance storage key for the global pause flag.
    IsPaused,
    /// Instance storage key for the StakeVault contract [`Address`].
    StakeVault,
}
