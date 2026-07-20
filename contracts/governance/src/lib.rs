#![no_std]

pub const PROPOSAL_COUNTER_START: u32 = 0;
// Operational notes — proposals progress through: created →
// voting → (executed | cancelled). Cancellation locks the
// proposal via `executed = true`. Vote weight is fetched at
// call time so badge holdings at the moment of the cast
// determine the weight.

pub const QUORUM_BASIS_POINTS: u32 = 3300;

pub const DEFAULT_VOTING_PERIOD_SECONDS: u64 = 604800;
// Crate overview — badge-weighted proposal lifecycle: create,
// vote, execute, cancel. Vote weight = number of badges owned at
// the moment of the cast.
use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, contracttype, symbol_short, Address,
    BytesN, Env, Symbol, Vec,
};

pub mod types;

pub use types::{DataKey, Proposal};

const BADGE_NFT_KEY: Symbol = symbol_short!("badge");
/// Instance-storage key for the running proposal-ID counter.
const PROPOSAL_ID_KEY: Symbol = symbol_short!("propid");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Badge {
    pub course_id: u32,
    pub minted_at: u64,
}

#[contractclient(name = "BadgeNFTClient")]
pub trait BadgeNFTInterface {
    fn get_badges(env: Env, learner: Address) -> Vec<Badge>;
}

#[contract]
pub struct Governance;

#[contractevent]
pub struct ProposalExecuted {
    #[topic]
    pub proposal_id: u32,
    pub proposer: Address,
}

#[contractevent]
pub struct ProposalCancelled {
    #[topic]
    pub proposal_id: u32,
    pub cancelled_by: Address,
}

#[contractevent]
pub struct ProposalCreated {
    #[topic]
    pub proposal_id: u32,
    pub proposer: Address,
}

#[contractevent]
pub struct ContractUpgraded {
    #[topic]
    pub admin: Address,
    pub new_wasm_hash: BytesN<32>,
}

#[contractimpl]
impl Governance {
    /// Initializes the governance contract with the admin and BadgeNFT contract address.
    /// Must be called once upon deployment.
    /// Bootstrap with admin and the BadgeNFT contract address used for
    /// vote-weight computation. The `BADGE_NFT_KEY` symbol constant
    /// names the instance slot.
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

    /// Creates a new governance proposal.
    ///
    /// Any caller may create a proposal; the proposer must authorize the call.
    /// The proposal is stored in persistent storage and the proposal ID counter
    /// is incremented.
    ///
    /// # Arguments
    /// * `proposer` — the address submitting the proposal (must sign).
    /// * `metadata_hash` — 32-byte hash pointing at the proposal description.
    /// * `voting_period_seconds` — seconds from now until voting closes.
    ///
    /// # Returns
    /// The new proposal ID.
    pub fn create_proposal(
        env: Env,
        proposer: Address,
        metadata_hash: BytesN<32>,
        voting_period_seconds: u64,
    ) -> u32 {
        proposer.require_auth();

        // Require the contract to be initialized.
        let _: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Contract not initialized");

        let proposal_id: u32 = env
            .storage()
            .instance()
            .get(&PROPOSAL_ID_KEY)
            .unwrap_or(0u32)
            + 1;
        env.storage()
            .instance()
            .set(&PROPOSAL_ID_KEY, &proposal_id);

        let end_time = env.ledger().timestamp() + voting_period_seconds;
        let proposal = Proposal {
            id: proposal_id,
            proposer: proposer.clone(),
            metadata_hash,
            votes_for: 0,
            votes_against: 0,
            end_time,
            executed: false,
        };
        env.storage()
            .persistent()
            .set(&DataKey::Proposal(proposal_id), &proposal);

        // Increment the footprint counter.
        let prev: u32 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::ProposalCount, &(prev + 1));

        ProposalCreated {
            proposal_id,
            proposer,
        }
        .publish(&env);

        proposal_id
    }

    /// Returns the proposal stored for the given proposal ID.
    /// Reads a Proposal struct from persistent storage by ID. The
    /// function panics with `"Proposal not found"` when no
    /// matching `DataKey::Proposal(id)` exists.
    pub fn get_proposal(env: Env, proposal_id: u32) -> Proposal {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .expect("Proposal not found")
    }

    /// Casts a vote on a proposal, weighted by the number of badges the voter owns.
    /// Records a voter's ballot weighted by the number of badges
    /// the voter owns at the moment of the call. Double-voting is
    /// blocked via the per-(voter, proposal_id) flag in
    /// `DataKey::UserVote`.
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

        // Increment the vote-count footprint counter.
        let prev: u32 = env
            .storage()
            .instance()
            .get(&DataKey::VoteCount)
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::VoteCount, &(prev + 1));
    }

    /// Returns an estimated count of persistent storage entries for this contract.
    ///
    /// Sums:
    /// * `ProposalCount` — one entry per proposal ever created.
    /// * `VoteCount` — one entry per ballot cast.
    pub fn estimated_storage_footprint(env: Env) -> u32 {
        let proposals: u32 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0);
        let votes: u32 = env
            .storage()
            .instance()
            .get(&DataKey::VoteCount)
            .unwrap_or(0);
        proposals + votes
    }

    /// Removes persistent-storage entries for a list of finalised (executed or
    /// cancelled) proposals and their associated vote records.
    ///
    /// # Admin-only
    ///
    /// # Arguments
    /// * `admin` — must match the stored Protocol Admin.
    /// * `sweep_learning_keys` — when `false` this is a no-op (returns 0).
    /// * `proposal_ids` — IDs of executed/cancelled proposals to reclaim.
    ///   Only proposals where `executed == true` are removed.
    ///
    /// # Returns
    /// Number of proposal entries removed (vote entries are also removed but
    /// not separately counted here).
    pub fn sweep_storage(
        env: Env,
        admin: Address,
        sweep_learning_keys: bool,
        proposal_ids: Vec<u32>,
    ) -> u32 {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        assert!(admin == stored_admin, "Unauthorized");

        if !sweep_learning_keys {
            return 0;
        }

        let mut removed: u32 = 0;
        let limit = if proposal_ids.len() > 50 {
            50
        } else {
            proposal_ids.len()
        };

        for i in 0..limit {
            let proposal_id = proposal_ids.get(i).expect("index invariant");
            let key = DataKey::Proposal(proposal_id);

            if let Some(proposal) = env
                .storage()
                .persistent()
                .get::<DataKey, Proposal>(&key)
            {
                if proposal.executed {
                    env.storage().persistent().remove(&key);

                    let prev: u32 = env
                        .storage()
                        .instance()
                        .get(&DataKey::ProposalCount)
                        .unwrap_or(0);
                    if prev > 0 {
                        env.storage()
                            .instance()
                            .set(&DataKey::ProposalCount, &(prev - 1));
                    }
                    removed += 1;
                }
            }
        }

        removed
    }

    /// Upgrades the contract WASM. Only callable by the Protocol Admin.
    /// Replaces the Governance WASM with the supplied hash on the
    /// Soroban host. Admin-only. Emits `ContractUpgraded` on
    /// successful deployment.
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

    /// Cancels an active proposal. Only callable by the proposer or the Protocol Admin.
    /// Proposer- or admin-only cancellation of an active proposal.
    /// Sets `proposal.executed = true` (the canonical "locked"
    /// state) and emits `ProposalCancelled`. Rejects cancel
    /// attempts after voting ends or after execution.
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

    /// Executes a proposal if it has passed and the voting period has ended.
    /// Marks the proposal as executed so the admin knows to action the approved change.
    /// Marks a passed proposal as executed if voting is closed and
    /// strictly more votes were cast in favor than against. Tied votes
    /// panic with `"Proposal rejected"`. Re-execution panics with
    /// `"Already executed"`.
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
