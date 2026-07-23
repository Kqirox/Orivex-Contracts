use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Token,
    Spender(Address),
    IsPaused,
    /// Pending two-step admin transfer. Holds a
    /// `contracts_common::two_step::PendingTransfer` while a transfer
    /// is in flight (issue #20).
    PendingAdmin,
}
