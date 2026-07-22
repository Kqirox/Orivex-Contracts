use soroban_sdk::contracterror;

/// Common error types for all Orivex contracts.
/// Replaces inline panic strings with a typed error enum.
#[contracterror]
#[derive(Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    /// The contract has already been initialized.
    AlreadyInitialized = 1,
    /// The contract has not been initialized yet.
    NotInitialized = 2,
    /// The caller is not authorized to perform this action.
    Unauthorized = 3,
    /// The contract is paused and cannot perform this action.
    ContractPaused = 4,
    /// The specified resource was not found.
    NotFound = 5,
    /// The provided amount is not valid (e.g., must be positive).
    InvalidAmount = 6,
    /// The operation has already been completed.
    AlreadyCompleted = 7,
    /// The requested resource already exists.
    AlreadyExists = 8,
}
