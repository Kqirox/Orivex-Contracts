//! Shared types for the Governance contract.

use soroban_sdk::{contracttype, Address, BytesN};

/// An on-chain governance proposal.
///
/// Stored in persistent storage under `DataKey::Proposal(id)`.
///
/// ## Lifecycle
///
/// ```text
/// created → voting open → (executed | cancelled)
/// ```
///
/// Cancellation reuses `executed = true` as the "locked" sentinel so that
/// both paths share the same re-entry guard in `execute_proposal` and
/// `cancel_proposal`.
///
/// ## Vote weight
///
/// Weight is fetched from the BadgeNFT contract at cast time, not at
/// proposal creation time.  Badge holdings at the moment of the vote
/// determine the voter's influence.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    /// Auto-assigned proposal ID.
    pub id: u32,
    /// Address that created the proposal.
    pub proposer: Address,
    /// 32-byte IPFS hash of the proposal description and rationale.
    pub metadata_hash: BytesN<32>,
    /// Cumulative badge-weighted votes in favour.
    pub votes_for: u32,
    /// Cumulative badge-weighted votes against.
    pub votes_against: u32,
    /// Unix timestamp (seconds) at which the voting window closes.
    pub end_time: u64,
    /// `true` once the proposal has been executed **or** cancelled.
    pub executed: bool,
}

/// Storage keys used by the Governance contract.
///
/// | Variant | Storage tier | Type | Description |
/// |---------|-------------|------|-------------|
/// | `Proposal(u32)` | Persistent | [`Proposal`] | Proposal record by ID |
/// | `UserVote(Address, u32)` | Persistent | `bool` | Double-vote guard |
/// | `Admin` | Instance | `Address` | Protocol admin address |
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Persistent storage key for a [`Proposal`] identified by `u32` ID.
    Proposal(u32),
    /// Persistent storage key preventing double-voting.
    /// Tuple: `(voter_address, proposal_id)`.
    UserVote(Address, u32),
    /// Instance storage key for the protocol admin [`Address`].
    Admin,
}
