use soroban_sdk::{contracttype, Address};

/// A soulbound badge representing course completion.
/// Shared between `badge-nft` and `governance` contracts.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Badge {
    pub course_id: u32,
    pub minted_at: u64,
}
