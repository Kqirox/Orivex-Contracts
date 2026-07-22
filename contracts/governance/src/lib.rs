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

use orivex_common::bump_persistent;

const BADGE_NFT_KEY: Symbol = symbol_short!("badge");

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
pub struct ContractUpgraded {
    #[topic]
    pub admin: Address,
    pub new_wasm_hash: BytesN<32>,
}

#[contractimpl]
impl Governance {
    /// Initializes the governance contract with the admin and BadgeNFT contract address.
    /// Must be called once upon deployment.
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

    /// Returns the proposal stored for the given proposal ID.
    /// Reads a Proposal struct from persistent storage by ID and bumps its TTL.
    pub fn get_proposal(env: Env, proposal_id: u32) -> Proposal {
        let key = DataKey::Proposal(proposal_id);
        let proposal: Proposal = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Proposal not found");
        bump_persistent(&env, &key);
        proposal
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

        let proposal_key = DataKey::Proposal(proposal_id);
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .expect("Proposal not found");
        bump_persistent(&env, &proposal_key);

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
            .set(&proposal_key, &proposal);
        bump_persistent(&env, &proposal_key);

        env.storage().persistent().set(&vote_key, &true);
        bump_persistent(&env, &vote_key);
    }

    /// Upgrades the contract WASM. Only callable by the Protocol Admin.
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
    /// Sets `proposal.executed = true` (the canonical "locked"
    /// state) and emits `ProposalCancelled`.
    pub fn cancel_proposal(env: Env, caller: Address, proposal_id: u32) {
        caller.require_auth();

        let proposal_key = DataKey::Proposal(proposal_id);
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .expect("Proposal not found");
        bump_persistent(&env, &proposal_key);

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
            .set(&proposal_key, &proposal);
        bump_persistent(&env, &proposal_key);

        ProposalCancelled {
            proposal_id,
            cancelled_by: caller,
        }
        .publish(&env);
    }

    /// Executes a proposal if it has passed and the voting period has ended.
    /// Marks a passed proposal as executed if voting is closed and
    /// strictly more votes were cast in favor than against.
    pub fn execute_proposal(env: Env, proposal_id: u32) {
        let proposal_key = DataKey::Proposal(proposal_id);
        let mut proposal: Proposal = env
            .storage()
            .persistent()
            .get(&proposal_key)
            .expect("Proposal not found");
        bump_persistent(&env, &proposal_key);

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
            .set(&proposal_key, &proposal);
        bump_persistent(&env, &proposal_key);

        ProposalExecuted {
            proposal_id,
            proposer: proposal.proposer,
        }
        .publish(&env);
    }
}

#[cfg(test)]
mod test;
