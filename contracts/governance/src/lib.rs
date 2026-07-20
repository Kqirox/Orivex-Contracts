//! # Governance Contract
//!
//! Badge-weighted proposal lifecycle for the Orivex protocol:
//! create → vote → execute | cancel.
//!
//! ## Operational notes
//!
//! * Vote weight equals the number of badges a voter holds **at the moment
//!   of casting** — not at proposal creation time.
//! * Cancellation sets `proposal.executed = true` (the canonical "locked"
//!   state), reusing the executed flag to block further state changes.
//! * Tied votes (`votes_for == votes_against`) are treated as rejected.
//!
//! ## Storage layout
//!
//! | Key | Tier | Type | Description |
//! |-----|------|------|-------------|
//! | `DataKey::Admin` | Instance | `Address` | Protocol admin |
//! | `BADGE_NFT_KEY` | Instance | `Address` | Wired BadgeNFT contract |
//! | `DataKey::Proposal(id)` | Persistent | [`Proposal`] | Proposal record |
//! | `DataKey::UserVote(addr, id)` | Persistent | `bool` | Double-vote guard |

#![no_std]

/// Starting value for the internal proposal ID counter.
pub const PROPOSAL_COUNTER_START: u32 = 0;

/// Quorum threshold in basis points (3300 bp = 33 %).
/// Currently informational; enforcement can be added to `execute_proposal`.
pub const QUORUM_BASIS_POINTS: u32 = 3300;

/// Default voting period in seconds (7 days = 604 800 s).
pub const DEFAULT_VOTING_PERIOD_SECONDS: u64 = 604800;
use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, contracttype, symbol_short, Address,
    BytesN, Env, Symbol, Vec,
};

pub mod types;

pub use types::{DataKey, Proposal};

const BADGE_NFT_KEY: Symbol = symbol_short!("badge");

/// Badge mirror type used for cross-contract vote-weight queries.
///
/// Matches the `Badge` struct in the BadgeNFT crate so the generated
/// `BadgeNFTClient` can deserialize the response without importing
/// the badge-nft crate directly.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Badge {
    /// Course ID the badge represents.
    pub course_id: u32,
    /// Ledger timestamp at mint time.
    pub minted_at: u64,
}

/// Cross-contract client interface for vote-weight queries.
///
/// Only `get_badges` is needed; the governance contract counts the
/// length of the returned vector to derive the voter's weight.
#[contractclient(name = "BadgeNFTClient")]
pub trait BadgeNFTInterface {
    /// Returns all badges currently held by `learner`.
    fn get_badges(env: Env, learner: Address) -> Vec<Badge>;
}

#[contract]
pub struct Governance;

/// Emitted when a proposal is successfully executed.
#[contractevent]
pub struct ProposalExecuted {
    /// ID of the executed proposal.
    #[topic]
    pub proposal_id: u32,
    /// Proposer address from the proposal record.
    pub proposer: Address,
}

/// Emitted when a proposal is cancelled.
#[contractevent]
pub struct ProposalCancelled {
    /// ID of the cancelled proposal.
    #[topic]
    pub proposal_id: u32,
    /// Address that triggered the cancellation.
    pub cancelled_by: Address,
}

/// Emitted when the contract WASM is upgraded.
#[contractevent]
pub struct ContractUpgraded {
    /// Admin who authorized the upgrade.
    #[topic]
    pub admin: Address,
    /// SHA-256 hash of the new WASM blob.
    pub new_wasm_hash: BytesN<32>,
}

#[contractimpl]
impl Governance {
    /// Initializes the governance contract with the admin and BadgeNFT contract address.
    ///
    /// Stores `admin` under `DataKey::Admin` and `badge_contract_address` under the
    /// `BADGE_NFT_KEY` symbol in instance storage. Must be called exactly once.
    ///
    /// # Arguments
    ///
    /// * `admin` — Protocol admin address (required auth).
    /// * `badge_contract_address` — Address of the wired BadgeNFT contract used
    ///   for vote-weight computation.
    ///
    /// # Panics
    ///
    /// * `"Already initialized"` — if `BADGE_NFT_KEY` already exists in instance storage.
    pub fn initialize(env: Env, admin: Address, badge_contract_address: Address) {
        if env.storage().instance().has(&BADGE_NFT_KEY) {
            panic!("Already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&BADGE_NFT_KEY, &badge_contract_address);
    }

    /// Returns the [`Proposal`] stored for the given ID.
    ///
    /// # Arguments
    ///
    /// * `proposal_id` — ID of the proposal to fetch.
    ///
    /// # Returns
    ///
    /// The [`Proposal`] struct stored under `DataKey::Proposal(proposal_id)`.
    ///
    /// # Panics
    ///
    /// * `"Proposal not found"` — if no matching record exists.
    pub fn get_proposal(env: Env, proposal_id: u32) -> Proposal {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .expect("Proposal not found")
    }

    /// Casts a vote on a proposal, weighted by the voter's current badge count.
    ///
    /// Vote weight equals `BadgeNFTClient::get_badges(voter).len()` at the time
    /// of the call. Double-voting is blocked by a per-`(voter, proposal_id)` flag
    /// stored in `DataKey::UserVote`.
    ///
    /// # Arguments
    ///
    /// * `voter` — Address casting the vote (required auth).
    /// * `proposal_id` — ID of the proposal to vote on.
    /// * `support` — `true` to vote for, `false` to vote against.
    ///
    /// # Panics
    ///
    /// * `"Already voted"` — if the voter has already cast a ballot on this proposal.
    /// * `"Contract not initialized"` — if the BadgeNFT address is absent.
    /// * `"Vote overflow"` — if the vote-count addition overflows `u32`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// client.cast_vote(&voter, &proposal_id, &true);
    /// let proposal = client.get_proposal(&proposal_id);
    /// assert!(proposal.votes_for > 0);
    /// ```
    pub fn cast_vote(env: Env, voter: Address, proposal_id: u32, support: bool) {
        voter.require_auth();

        let vote_key = DataKey::UserVote(voter.clone(), proposal_id);
        assert!(!env.storage().persistent().has(&vote_key), "Already voted");

        let badge_contract_address: Address = env
            .storage()
            .instance()
            .get(&BADGE_NFT_KEY)
            .expect("Contract not initialized");
        let badge_client = BadgeNFTClient::new(&env, &badge_contract_address);
        let weight = badge_client.get_badges(&voter).len();

        let mut proposal = Self::get_proposal(env.clone(), proposal_id);
        if support {
            proposal.votes_for = proposal
                .votes_for
                .checked_add(weight)
                .expect("Vote overflow");
        } else {
            proposal.votes_against = proposal
                .votes_against
                .checked_add(weight)
                .expect("Vote overflow");
        }

        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);
        env.storage().persistent().set(&vote_key, &true);
    }

    /// Upgrades the contract WASM to a new hash. Only callable by the protocol admin.
    ///
    /// Replaces the Governance WASM on the Soroban host and emits
    /// [`ContractUpgraded`] on success.
    ///
    /// # Arguments
    ///
    /// * `admin` — Must equal the stored admin (required auth).
    /// * `new_wasm_hash` — SHA-256 hash of the replacement WASM blob.
    ///
    /// # Panics
    ///
    /// * `"Not initialized"` — if the contract has not been initialized.
    /// * `"Unauthorized"` — if `admin ≠ stored admin`.
    pub fn upgrade_contract(env: Env, admin: Address, new_wasm_hash: BytesN<32>) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        assert!(admin == stored_admin, "Unauthorized");

        env.deployer()
            .update_current_contract_wasm(new_wasm_hash.clone());

        ContractUpgraded {
            admin,
            new_wasm_hash,
        }
        .publish(&env);
    }

    /// Cancels an active proposal. Callable by the proposer or the protocol admin.
    ///
    /// Sets `proposal.executed = true` (the canonical "locked" state) and emits
    /// [`ProposalCancelled`]. Attempts to cancel after the voting window ends,
    /// or after execution, both panic.
    ///
    /// # Arguments
    ///
    /// * `caller` — Proposer or admin address (required auth).
    /// * `proposal_id` — ID of the proposal to cancel.
    ///
    /// # Panics
    ///
    /// * `"Proposal not found"` — if no matching record exists.
    /// * `"Not initialized"` — if the contract has not been initialized.
    /// * `"Unauthorized"` — if `caller` is neither the proposer nor the admin.
    /// * `"Voting ended"` — if `ledger.timestamp() >= proposal.end_time`.
    /// * `"Already executed"` — if `proposal.executed` is already `true`.
    pub fn cancel_proposal(env: Env, caller: Address, proposal_id: u32) {
        caller.require_auth();

        let mut proposal = Self::get_proposal(env.clone(), proposal_id);

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");

        assert!(
            caller == proposal.proposer || caller == stored_admin,
            "Unauthorized"
        );
        assert!(env.ledger().timestamp() < proposal.end_time, "Voting ended");
        assert!(!proposal.executed, "Already executed");

        proposal.executed = true;
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        ProposalCancelled {
            proposal_id,
            cancelled_by: caller,
        }
        .publish(&env);
    }

    /// Executes a passed proposal after its voting period has ended.
    ///
    /// Marks the proposal as executed so the admin knows to action the approved
    /// change off-chain. Strictly more `votes_for` than `votes_against` is
    /// required; tied or failing votes panic.
    ///
    /// # Arguments
    ///
    /// * `proposal_id` — ID of the proposal to execute.
    ///
    /// # Panics
    ///
    /// * `"Proposal not found"` — if no matching record exists.
    /// * `"Voting still active"` — if `ledger.timestamp() <= proposal.end_time`.
    /// * `"Proposal rejected"` — if `votes_for <= votes_against`.
    /// * `"Already executed"` — if `proposal.executed` is already `true`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // advance ledger past end_time, then:
    /// client.execute_proposal(&proposal_id);
    /// assert!(client.get_proposal(&proposal_id).executed);
    /// ```
    pub fn execute_proposal(env: Env, proposal_id: u32) {
        let mut proposal = Self::get_proposal(env.clone(), proposal_id);

        assert!(
            env.ledger().timestamp() > proposal.end_time,
            "Voting still active"
        );
        assert!(
            proposal.votes_for > proposal.votes_against,
            "Proposal rejected"
        );
        assert!(!proposal.executed, "Already executed");

        proposal.executed = true;
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        ProposalExecuted {
            proposal_id,
            proposer: proposal.proposer,
        }
        .publish(&env);
    }
}

#[cfg(test)]
mod test;
