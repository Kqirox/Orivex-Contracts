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

/// Schema used before VERSION 1 was introduced (v0 layout).
/// Wire-compatible with `Proposal`; kept for documentation clarity.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProposalV0 {
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
    /// Monotonically increasing schema version stored in instance storage.
    /// 0  = pre-versioning (no Version key present).
    /// 1  = current schema (this build).
    Version,
}
