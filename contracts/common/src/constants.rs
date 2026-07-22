/// Platform fee in basis points (15% = 1500 basis points).
/// Used by both `quest-engine` and `reward-pool`.
pub const PLATFORM_FEE_BASIS_POINTS: u32 = 1500;

/// Default lock/voting period in seconds (7 days = 604800).
/// Used by both `stake-vault` and `governance`.
pub const DEFAULT_PERIOD_SECONDS: u64 = 604800;

/// Maximum number of badges per learner.
/// Used by `badge-nft`.
pub const MAX_BADGES_PER_LEARNER: u32 = 64;

/// Default reward token decimals (7 = USDC).
/// Used by `reward-pool`.
pub const REWARD_TOKEN_DECIMALS: u32 = 7;

/// Maximum number of approved spenders.
/// Used by `reward-pool`.
pub const MAX_SPENDERS: u32 = 256;

/// Minimum payout amount (must be > 0).
/// Used by `reward-pool`.
pub const MIN_PAYOUT_AMOUNT: i128 = 1;

/// Maximum quest reward amount.
/// Used by `quest-engine`.
pub const MAX_QUEST_REWARD: i128 = 1_000_000_000_000_000;

/// Default total modules bound for courses.
/// Used by `course-registry`.
pub const DEFAULT_TOTAL_MODULES_BOUND: u32 = 1000;

/// Base reward amount for course completion (10 USDC with 7 decimals).
/// Used by `course-registry`.
pub const BASE_REWARD_AMOUNT: i128 = 10_000_000;
