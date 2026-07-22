#![no_std]

pub const BUILD_QUEST_PREFIX: &str = "build";

pub const EXPLORE_QUEST_PREFIX: &str = "explore";
// Operational notes — review paths cross-call
// `StakeVault.get_multiplier` for payout scaling. Explore-quest
// payouts route via `RewardPool.distribute_reward` (which
// requires the QuestEngine to be whitelisted on the
// RewardPool).

pub const MAX_QUEST_REWARD: i128 = 1_000_000_000_000_000;

pub const PLATFORM_FEE_BASIS_POINTS: u32 = 1500;
// Crate overview — Build and Explore quests. Build quests are
// employer-funded and reviewed per submission. Explore quests are
// admin-verified and rewarded out of the RewardPool.

pub mod types;
use types::{DataKey, Quest, QuestType, Submission, SubmissionStatus};

use orivex_common::bump_persistent;

use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, token, Address, BytesN, Env, Vec,
};

#[contractclient(name = "StakeVaultClient")]
pub trait StakeVaultInterface {
    fn get_multiplier(env: Env, learner: Address) -> u32;
}

#[contractclient(name = "RewardPoolClient")]
pub trait RewardPoolInterface {
    fn distribute_reward(env: Env, caller: Address, learner: Address, amount: i128);
}

#[contractevent]
pub struct QuestCreated {
    #[topic]
    pub employer: Address,
    #[topic]
    pub quest_id: u32,
    pub reward_amount: i128,
}

#[contractevent]
pub struct ProofSubmitted {
    #[topic]
    pub learner: Address,
    #[topic]
    pub quest_id: u32,
    pub proof_hash: BytesN<32>,
}

#[contractevent]
pub struct SubmissionReviewed {
    #[topic]
    pub employer: Address,
    #[topic]
    pub learner: Address,
    #[topic]
    pub quest_id: u32,
    pub approved: bool,
}

#[contractevent]
pub struct QuestRefunded {
    #[topic]
    pub employer: Address,
    #[topic]
    pub quest_id: u32,
    pub amount: i128,
}

#[contractevent]
pub struct BatchReviewed {
    #[topic]
    pub employer: Address,
    #[topic]
    pub quest_id: u32,
    pub approved_count: u32,
}

#[contractevent]
pub struct ContractUpgraded {
    #[topic]
    pub admin: Address,
    pub new_wasm_hash: BytesN<32>,
}

#[contractevent]
pub struct ExploreQuestVerified {
    #[topic]
    pub admin: Address,
    #[topic]
    pub learner: Address,
    #[topic]
    pub quest_id: u32,
    pub amount: i128,
}

#[contract]
pub struct QuestEngineContract;

#[contractimpl]
impl QuestEngineContract {
    /// Initializes the QuestEngine contract with the token address and admin.
    pub fn initialize(
        env: Env,
        admin: Address,
        token: Address,
        reward_pool: Address,
        stake_vault: Address,
    ) {
        if env.storage().instance().has(&DataKey::Token) {
            panic!("Already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::Token, &token);
        env.storage()
            .instance()
            .set(&DataKey::RewardPool, &reward_pool);
        env.storage()
            .instance()
            .set(&DataKey::StakeVault, &stake_vault);
        env.storage().instance().set(&DataKey::QuestCounter, &0u32);
    }

    /// Toggles the pause state of the contract (emergency circuit breaker).
    pub fn set_pause(env: Env, admin: Address, status: bool) {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");

        if admin != stored_admin {
            panic!("Unauthorized");
        }

        admin.require_auth();

        env.storage().instance().set(&DataKey::IsPaused, &status);
    }

    /// Allows an employer to lock USDC directly in the QuestEngine contract.
    /// The full `reward_amount` is locked at create time; review actions
    /// later split it 85/15 between learner and reward-pool.
    pub fn create_build_quest(
        env: Env,
        employer: Address,
        reward_amount: i128,
        metadata_hash: BytesN<32>,
    ) -> u32 {
        employer.require_auth();

        let token_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("Not initialized");
        let token_client = token::Client::new(&env, &token_address);

        token_client.transfer(&employer, env.current_contract_address(), &reward_amount);

        let mut quest_id: u32 = env
            .storage()
            .instance()
            .get(&DataKey::QuestCounter)
            .unwrap_or(0);
        quest_id += 1;
        env.storage()
            .instance()
            .set(&DataKey::QuestCounter, &quest_id);

        let quest = Quest {
            employer: employer.clone(),
            reward_amount,
            quest_type: QuestType::Build,
            metadata_hash,
            active: true,
        };

        let quest_key = DataKey::Quest(quest_id);
        env.storage()
            .persistent()
            .set(&quest_key, &quest);
        bump_persistent(&env, &quest_key);

        QuestCreated {
            employer,
            quest_id,
            reward_amount,
        }
        .publish(&env);

        quest_id
    }

    /// Creates an Explore Quest that will be funded by the RewardPool.
    /// Admin-only. The employer field is set to the admin so downstream
    /// payout flows can route via RewardPool's `distribute_reward`.
    pub fn create_explore_quest(
        env: Env,
        admin: Address,
        reward_amount: i128,
        metadata_hash: BytesN<32>,
    ) -> u32 {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        assert!(admin == stored_admin, "Unauthorized");

        let mut quest_id: u32 = env
            .storage()
            .instance()
            .get(&DataKey::QuestCounter)
            .unwrap_or(0);
        quest_id += 1;
        env.storage()
            .instance()
            .set(&DataKey::QuestCounter, &quest_id);

        let quest = Quest {
            employer: admin.clone(),
            reward_amount,
            quest_type: QuestType::Explore,
            metadata_hash,
            active: true,
        };

        let quest_key = DataKey::Quest(quest_id);
        env.storage()
            .persistent()
            .set(&quest_key, &quest);
        bump_persistent(&env, &quest_key);

        QuestCreated {
            employer: admin,
            quest_id,
            reward_amount,
        }
        .publish(&env);

        quest_id
    }

    /// Returns a quest by its ID. Returns `None` when not found.
    pub fn get_quest(env: Env, quest_id: u32) -> Option<Quest> {
        let quest_key = DataKey::Quest(quest_id);
        let quest: Option<Quest> = env.storage().persistent().get(&quest_key);
        if quest.is_some() {
            bump_persistent(&env, &quest_key);
        }
        quest
    }

    /// Stores a learner's proof hash for the given build quest.
    /// The associated quest must be active and of `QuestType::Build`.
    /// Re-submission for the same (learner, quest) pair panics with
    /// `"Submission already exists"`.
    pub fn submit_proof(env: Env, learner: Address, quest_id: u32, proof_hash: BytesN<32>) {
        learner.require_auth();

        let quest_key = DataKey::Quest(quest_id);
        let quest: Quest = env
            .storage()
            .persistent()
            .get(&quest_key)
            .expect("Quest not found");
        bump_persistent(&env, &quest_key);

        if !quest.active {
            panic!("Quest is not active");
        }
        if quest.quest_type != QuestType::Build {
            panic!("Only Build quests accept submissions");
        }

        let submission_key = DataKey::Submission(learner.clone(), quest_id);

        if env.storage().persistent().has(&submission_key) {
            panic!("Submission already exists");
        }

        let submission = Submission {
            proof_hash: proof_hash.clone(),
            status: SubmissionStatus::Pending,
        };
        env.storage().persistent().set(&submission_key, &submission);
        bump_persistent(&env, &submission_key);

        ProofSubmitted {
            learner,
            quest_id,
            proof_hash,
        }
        .publish(&env);
    }

    /// Returns a submission by learner and quest ID. Returns `None` if not found.
    pub fn get_submission(env: Env, learner: Address, quest_id: u32) -> Option<Submission> {
        let submission_key = DataKey::Submission(learner, quest_id);
        let submission: Option<Submission> = env.storage().persistent().get(&submission_key);
        if submission.is_some() {
            bump_persistent(&env, &submission_key);
        }
        submission
    }

    /// Approves or rejects a single submission, applying the staking multiplier
    /// from the configured StakeVault. The boosted learner payout is capped at
    /// the available post-fee balance.
    pub fn review_submission(
        env: Env,
        employer: Address,
        learner: Address,
        quest_id: u32,
        approve: bool,
    ) {
        let is_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::IsPaused)
            .unwrap_or(false);
        assert!(!is_paused, "Contract is paused");

        employer.require_auth();

        let quest_key = DataKey::Quest(quest_id);
        let quest: Quest = env
            .storage()
            .persistent()
            .get(&quest_key)
            .expect("Quest not found");
        bump_persistent(&env, &quest_key);

        if quest.employer != employer {
            panic!("Only the quest employer can review submissions");
        }

        let submission_key = DataKey::Submission(learner.clone(), quest_id);
        let mut submission: Submission = env
            .storage()
            .persistent()
            .get(&submission_key)
            .expect("Submission not found");
        bump_persistent(&env, &submission_key);

        if submission.status != SubmissionStatus::Pending {
            panic!("Submission is not pending review");
        }

        if approve {
            let token_address: Address = env
                .storage()
                .instance()
                .get(&DataKey::Token)
                .expect("Not initialized");
            let token_client = token::Client::new(&env, &token_address);

            let fee = (quest.reward_amount * 15) / 100;
            let base_learner_amount = quest.reward_amount - fee;

            let stake_vault_address: Address = env
                .storage()
                .instance()
                .get(&DataKey::StakeVault)
                .expect("Not initialized");
            let stake_vault_client = StakeVaultClient::new(&env, &stake_vault_address);
            let multiplier = stake_vault_client.get_multiplier(&learner);

            let calculated_boost = (base_learner_amount * multiplier as i128) / 100;
            let learner_amount = if calculated_boost > base_learner_amount {
                base_learner_amount
            } else {
                calculated_boost
            };

            let reward_pool: Address = env
                .storage()
                .instance()
                .get(&DataKey::RewardPool)
                .expect("Not initialized");

            token_client.transfer(&env.current_contract_address(), &reward_pool, &fee);
            token_client.transfer(&env.current_contract_address(), &learner, &learner_amount);

            submission.status = SubmissionStatus::Approved;
        } else {
            submission.status = SubmissionStatus::Rejected;
        }

        env.storage().persistent().set(&submission_key, &submission);
        bump_persistent(&env, &submission_key);

        SubmissionReviewed {
            employer,
            learner,
            quest_id,
            approved: approve,
        }
        .publish(&env);
    }

    /// Employer-only cancellation of an in-flight Build quest.
    /// Returns the locked `reward_amount` to the employer and marks the quest inactive.
    pub fn refund_quest(env: Env, employer: Address, quest_id: u32) {
        employer.require_auth();

        let quest_key = DataKey::Quest(quest_id);
        let mut quest: Quest = env
            .storage()
            .persistent()
            .get(&quest_key)
            .expect("Quest not found");
        bump_persistent(&env, &quest_key);

        if quest.employer != employer {
            panic!("Unauthorized");
        }
        if !quest.active {
            panic!("Quest already inactive");
        }

        quest.active = false;
        env.storage()
            .persistent()
            .set(&quest_key, &quest);
        bump_persistent(&env, &quest_key);

        let token_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("Not initialized");
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(
            &env.current_contract_address(),
            &employer,
            &quest.reward_amount,
        );

        QuestRefunded {
            employer,
            quest_id,
            amount: quest.reward_amount,
        }
        .publish(&env);
    }

    /// Approves a vector of learner submissions against a single quest in one transaction.
    /// Emits individual `SubmissionReviewed` events and a `BatchReviewed` summary.
    pub fn batch_review_submissions(
        env: Env,
        employer: Address,
        quest_id: u32,
        learners: Vec<Address>,
    ) {
        let is_paused: bool = env
            .storage()
            .instance()
            .get(&DataKey::IsPaused)
            .unwrap_or(false);
        assert!(!is_paused, "Contract is paused");

        employer.require_auth();

        let quest_key = DataKey::Quest(quest_id);
        let quest: Quest = env
            .storage()
            .persistent()
            .get(&quest_key)
            .expect("Quest not found");
        bump_persistent(&env, &quest_key);

        if quest.employer != employer {
            panic!("Only the quest employer can review submissions");
        }

        let token_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::Token)
            .expect("Not initialized");
        let token_client = token::Client::new(&env, &token_address);

        let reward_pool: Address = env
            .storage()
            .instance()
            .get(&DataKey::RewardPool)
            .expect("Not initialized");

        let mut approved_count: u32 = 0;
        for learner in learners.iter() {
            let submission_key = DataKey::Submission(learner.clone(), quest_id);
            let mut submission: Submission = env
                .storage()
                .persistent()
                .get(&submission_key)
                .expect("Submission not found");
            bump_persistent(&env, &submission_key);

            if submission.status != SubmissionStatus::Pending {
                panic!("Submission is not pending review");
            }

            let fee = (quest.reward_amount * 15) / 100;
            let learner_amount = quest.reward_amount - fee;

            token_client.transfer(&env.current_contract_address(), &reward_pool, &fee);
            token_client.transfer(&env.current_contract_address(), &learner, &learner_amount);

            submission.status = SubmissionStatus::Approved;
            env.storage().persistent().set(&submission_key, &submission);
            bump_persistent(&env, &submission_key);

            SubmissionReviewed {
                employer: employer.clone(),
                learner,
                quest_id,
                approved: true,
            }
            .publish(&env);

            approved_count += 1;
        }

        BatchReviewed {
            employer,
            quest_id,
            approved_count,
        }
        .publish(&env);
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

    /// Admin-only confirmation that a learner completed an off-chain action.
    /// Triggers a cross-contract `distribute_reward` call into the configured RewardPool.
    /// The QuestEngine must be whitelisted as an approved spender on RewardPool.
    pub fn verify_explore_quest(env: Env, admin: Address, learner: Address, quest_id: u32) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        assert!(admin == stored_admin, "Unauthorized");

        let quest_key = DataKey::Quest(quest_id);
        let quest: Quest = env
            .storage()
            .persistent()
            .get(&quest_key)
            .expect("Quest not found");
        bump_persistent(&env, &quest_key);

        assert!(
            quest.quest_type == QuestType::Explore,
            "Not an Explore quest"
        );

        let reward_pool_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::RewardPool)
            .expect("Not initialized");
        let reward_pool_client = RewardPoolClient::new(&env, &reward_pool_address);

        reward_pool_client.distribute_reward(
            &env.current_contract_address(),
            &learner,
            &quest.reward_amount,
        );

        ExploreQuestVerified {
            admin,
            learner,
            quest_id,
            amount: quest.reward_amount,
        }
        .publish(&env);
    }
}

#[cfg(test)]
mod test;
