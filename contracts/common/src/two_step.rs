#![no_std]

//! # Two-Step Transfer Helper (Issue #20)
//!
//! Shared types and events for two-step admin / role transfers across all
//! Orivex contracts. Each role in each contract follows the same triplet:
//!
//! 1. `propose_new_<role>(env, current_admin, proposed)` — current admin (or
//!    the role's current holder for self-keyed propose methods) starts the
//!    transfer. A `PendingTransfer` is written to persistent storage under
//!    a contract-defined key and `TransferProposed` is emitted.
//! 2. `accept_<role>(env, acceptor)` — only the proposed address may accept.
//!    The live storage slot is overwritten and the pending record is
//!    cleared; `TransferAccepted` is emitted.
//! 3. `cancel_<role>(env, caller)` — only the proposed address or the
//!    current admin may cancel. The pending record is cleared and
//!    `TransferCancelled` is emitted.
//!
//! ## Timelock
//!
//! This crate ships a **soft timelock** (see Issue #20 acceptance criteria):
//! acceptance and cancellation are both immediate. Off-chain monitors are
//! expected to alert on `TransferProposed` so communities can react before
//! the proposed address calls `accept_*`. A hard-timelock upgrade is a
//! straightforward follow-up that adds a `delay_seconds` field to
//! `PendingTransfer` and a check in `accept_<role>_*` callers.

use soroban_sdk::{contractevent, contracttype, Address};

/// A pending two-step transfer proposal.
///
/// Stored in the calling contract's persistent storage under a key of the
/// contract's choice (typically `DataKey::Pending<Role>`). The contract
/// reads it back during the corresponding `accept_<role>` to authorize the
/// final write and clears the record on success.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingTransfer {
    /// Address proposed for the role.
    pub proposed: Address,
    /// Ledger timestamp when the proposal was made.
    pub proposed_at: u64,
}

/// Emitted when the current role holder proposes a new address. Topics:
/// the current value and the proposed value. Data: timestamp.
#[contractevent]
pub struct TransferProposed {
    #[topic]
    pub current: Address,
    #[topic]
    pub proposed: Address,
    pub proposed_at: u64,
}

/// Emitted when the proposed address accepts and the live value updates.
/// Topic: the new value (the address that just became live).
#[contractevent]
pub struct TransferAccepted {
    #[topic]
    pub new_value: Address,
}

/// Emitted when a pending transfer is cancelled before acceptance.
/// Topics: the canceller and the address that was proposed.
#[contractevent]
pub struct TransferCancelled {
    #[topic]
    pub cancelled_by: Address,
    #[topic]
    pub was_proposed: Address,
}
