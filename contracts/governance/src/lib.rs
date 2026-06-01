#![no_std]
use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, contracttype, symbol_short, Address,
    Env, Symbol, Vec,
};

pub mod types;

pub use types::{DataKey, Proposal};

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

#[contractimpl]
impl Governance {
    /// Initializes the governance contract with the BadgeNFT contract address.
    /// Must be called once upon deployment.
    pub fn initialize(env: Env, badge_contract_address: Address) {
        if env.storage().instance().has(&BADGE_NFT_KEY) {
            panic!("Already initialized");
        }

        env.storage()
            .instance()
            .set(&BADGE_NFT_KEY, &badge_contract_address);
    }

    /// Returns the proposal stored for the given proposal ID.
    pub fn get_proposal(env: Env, proposal_id: u32) -> Proposal {
        env.storage()
            .persistent()
            .get(&DataKey::Proposal(proposal_id))
            .expect("Proposal not found")
    }

    /// Casts a vote on a proposal, weighted by the number of badges the voter owns.
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

    /// Executes a proposal if it has passed and the voting period has ended.
    /// Marks the proposal as executed so the admin knows to action the approved change.
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
