use soroban_sdk::{contracttype, Address, BytesN};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u32,
    pub proposer: Address,
    pub metadata_hash: BytesN<32>,
    pub votes_for: u32,
    pub votes_against: u32,
    pub end_time: u64,
    pub executed: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Proposal(u32),
    UserVote(Address, u32),
    Admin,
    // ── Two-step transfer slots (Issue #20) ────────────────
    /// Pending Admin transfer.
    PendingAdmin,
    /// Pending BadgeContractAddress transfer.
    PendingBadgeContract,
}
