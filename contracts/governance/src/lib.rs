#![no_std]
use badge_nft::BadgeNFTClient;
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};

pub mod types;

pub use types::{DataKey, Proposal};

const BADGE_NFT_KEY: Symbol = symbol_short!("badge");

#[contract]
pub struct Governance;

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
}

#[cfg(test)]
mod test;
