use soroban_sdk::{contracttype, Address};

/// Basis-points multiplier tiers returned by `StakeVault::get_multiplier`.
///
/// Each variant encodes the multiplier as an integer number of basis points
/// (hundredths of 1×), so dividing by 100 gives the actual scaling factor:
///
/// | Variant | Raw BPS | Effective multiplier |
/// |---------|---------|----------------------|
/// | `None`  | 100     | 1.0×                 |
/// | `Low`   | 120     | 1.2×                 |
/// | `High`  | 200     | 2.0×                 |
///
/// Cross-contract callers should always use [`MultiplierBps::as_bps`] rather
/// than matching on the raw discriminant to stay insulated from future tier
/// additions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MultiplierBps {
    /// No stake bonus — 100 bps (1.0×). Default for unstaked learners.
    None,
    /// Low-tier stake bonus — 120 bps (1.2×). Requires ≥ 100 tokens staked.
    Low,
    /// High-tier stake bonus — 200 bps (2.0×). Requires ≥ 500 tokens staked.
    High,
}

impl MultiplierBps {
    /// Returns the raw basis-points value for this tier.
    ///
    /// Use this when performing arithmetic:
    /// ```ignore
    /// let payout = (base_amount * multiplier.as_bps() as i128) / 100;
    /// ```
    pub fn as_bps(&self) -> u32 {
        match self {
            MultiplierBps::None => 100,
            MultiplierBps::Low => 120,
            MultiplierBps::High => 200,
        }
    }
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StakeInfo {
    pub amount: i128,
    pub lock_timestamp: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Token,
    UserStake(Address),
}
