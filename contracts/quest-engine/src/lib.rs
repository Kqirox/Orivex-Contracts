//! # Quest Engine Contract
//!
//! Build and Explore quest lifecycle for the Orivex protocol.
//!
//! * **Build quests** — employer-funded bounties. Reward tokens are locked
//!   in the contract at creation; the employer reviews submissions and splits
//!   the locked amount 85 % (learner) / 15 % (platform fee → RewardPool).
//!   The learner's share is further scaled by their [`StakeVault`] multiplier.
//!
//! * **Explore quests** — admin-verified off-chain actions. No tokens are
//!   locked up front; on verification the contract triggers
//!   `RewardPool::distribute_reward` directly.
//!
//! ## Operational notes
//!
//! * Review paths call `StakeVault::get_multiplier` for payout scaling.
//! * Explore-quest payouts route via `RewardPool::distribute_reward`, which
//!   requires the QuestEngine to be whitelisted as an approved spender.
//! * The global [`IsPaused`](types::DataKey::IsPaused) flag blocks
//!   `review_submission` and `batch_review_submissions` when set.

#![no_std]

/// Storage-key prefix constant for Build quest identifiers (informational).
pub const BUILD_QUEST_PREFIX: &str = "build";

/// Storage-key prefix constant for Explore quest identifiers (informational).
pub const EXPLORE_QUEST_PREFIX: &str = "explore";

/// Hard cap on the reward amount for a single quest (in token base units).
pub const MAX_QUEST_REWARD: i128 = 1_000_000_000_000_000;

/// Platform fee expressed in basis points (1500 bp = 15 %).
/// Deducted from Build-quest `reward_amount` before the learner payout.
pub const PLATFORM_FEE_BASIS_POINTS: u32 = 1500;

pub mod types;
use types::{DataKey, Quest, QuestType, Submission, SubmissionStatus};

use soroban_sdk::{
    contract, contractclient, contractevent, contractimpl, token, Address, BytesN, Env, Vec,
};

/// Cross-contract client interface for the StakeVault contract.
///
/// Used by `review_submission` to fetch the learner's payout multiplier
/// without importing the StakeVault crate directly.
#[contractclient(name = "StakeVaultClient")]
pub trait StakeVaultInterface {
    /// Returns the basis-points multiplier for `learner`'s current stake.
    fn get_multiplier(env: Env, learner: Address) -> u32;
}

/// Cross-contract client interface for the RewardPool contract.
///
/// Used by `verify_explore_quest` to trigger USDC payouts from the pool.
#[contractclient(name = "RewardPoolClient")]
pub trait RewardPoolInterface {
    /// Distributes `amount` tokens to `learner` from the pool.
    fn distribute_reward(env: Env, caller: Address, learner: Address, amount: i128);
}

/// Emitted when a new quest is created (Build or Explore).
#[contractevent]
pub struct QuestCreated {
    /// Address of the employer (Build) or admin (Explore) who created the quest.
    #[topic]
    pub employer: Address,
    /// Auto-assigned quest ID.
    #[topic]
    pub quest_id: u32,
    /// Tokens locked (Build) or earmarked from RewardPool (Explore).
    pub reward_amount: i128,
}

/// Emitted when a learner submits proof for a Build quest.
#[contractevent]
pub struct ProofSubmitted {
    /// Learner who submitted the proof.
    #[topic]
    pub learner: Address,
    /// Quest the proof targets.
    #[topic]
    pub quest_id: u32,
    /// IPFS or on-chain hash of the proof artifact.
    pub proof_hash: BytesN<32>,
}

/// Emitted after an employer approves or rejects a single submission.
#[contractevent]
pub struct SubmissionReviewed {
    /// Employer who performed the review.
    #[topic]
    pub employer: Address,
    /// Learner whose submission was reviewed.
    #[topic]
    pub learner: Address,
    /// Quest the submission belongs to.
    #[topic]
    pub quest_id: u32,
    /// `true` if approved, `false` if rejected.
    pub approved: bool,
}

/// Emitted when an employer cancels a Build quest and reclaims the locked funds.
#[contractevent]
pub struct QuestRefunded {
    /// Employer who initiated the refund.
    #[topic]
    pub employer: Address,
    /// Quest that was cancelled.
    #[topic]
    pub quest_id: u32,
    /// Amount returned to the employer.
    pub amount: i128,
}

/// Emitted at the end of a successful `batch_review_submissions` call.
#[contractevent]
pub struct BatchReviewed {
    /// Employer who performed the batch review.
    #[topic]
    pub employer: Address,
    /// Quest all submissions belonged to.
    #[topic]
    pub quest_id: u32,
    /// Number of submissions approved in this batch.
    pub approved_count: u32,
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

/// Emitted when the admin verifies an Explore quest completion and triggers a payout.
#[contractevent]
pub struct ExploreQuestVerified {
    /// Admin who verified the completion.
    #[topic]
    pub admin: Address,
    /// Learner who completed the off-chain action.
    #[topic]
    pub learner: Address,
    /// Explore quest that was verified.
    #[topic]
    pub quest_id: u32,
    /// Reward amount distributed from the RewardPool.
    pub amount: i128,
}

#[contract]
pub struct QuestEngineContract;

#[contractimpl]
impl QuestEngineContract {
    /// Initializes the QuestEngine with its admin, token, RewardPool, and StakeVault addresses.
    ///
    /// Stores all four addresses in instance storage and sets the quest counter to zero.
    /// Must be called exactly once after deployment.
    ///
    /// # Arguments
    ///
    /// * `admin` — Protocol admin (required auth).
    /// * `token` — USDC token address used for Build-quest payouts.
    /// * `reward_pool` — Address of the wired RewardPool contract.
    /// * `stake_vault` — Address of the wired StakeVault contract.
    ///
    /// # Panics
    ///
    /// * `"Already initialized"` — if `DataKey::Token` is already set.
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

    /// Toggles the global pause state of the contract.
    ///
    /// When paused, `review_submission` and `batch_review_submissions` panic
    /// with `"Contract is paused"`. Admin-only circuit-breaker for incidents.
    ///
    /// # Arguments
    ///
    /// * `admin` — Must equal the stored admin (required auth).
    /// * `status` — `true` to pause, `false` to unpause.
    ///
    /// # Panics
    ///
    /// * `"Not initialized"` — if the contract has not been initialized.
    /// * `"Unauthorized"` — if `admin ≠ stored admin`.
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

    /// Creates a Build quest, locking `reward_amount` tokens from the employer.
    ///
    /// The full reward is transferred from the employer's wallet into the
    /// QuestEngine contract at creation time. On approval the amount is split
    /// 85 % (learner, adjusted by staking multiplier) / 15 % (platform fee).
    ///
    /// # Arguments
    ///
    /// * `employer` — Address funding the quest (required auth).
    /// * `reward_amount` — Tokens to lock; transferred immediately.
    /// * `metadata_hash` — 32-byte IPFS hash of the quest description.
    ///
    /// # Returns
    ///
    /// The auto-assigned `u32` quest ID.
    ///
    /// # Panics
    ///
    /// * `"Not initialized"` — if the contract has not been initialized.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let id = client.create_build_quest(&employer, &1000_0000000i128, &hash);
    /// assert!(client.get_quest(&id).is_some());
    /// ```
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

        env.storage()
            .persistent()
            .set(&DataKey::Quest(quest_id), &quest);

        QuestCreated {
            employer,
            quest_id,
            reward_amount,
        }
        .publish(&env);

        quest_id
    }

    /// Creates an Explore quest funded by the RewardPool on admin verification.
    ///
    /// No tokens are locked in the QuestEngine. When the admin later calls
    /// `verify_explore_quest`, the reward is pulled from the configured
    /// RewardPool via `distribute_reward`.
    ///
    /// # Arguments
    ///
    /// * `admin` — Must equal the stored admin (required auth).
    /// * `reward_amount` — Amount the RewardPool will pay on verification.
    /// * `metadata_hash` — 32-byte IPFS hash of the quest description.
    ///
    /// # Returns
    ///
    /// The auto-assigned `u32` quest ID.
    ///
    /// # Panics
    ///
    /// * `"Not initialized"` — if the contract has not been initialized.
    /// * `"Unauthorized"` — if `admin ≠ stored admin`.
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

        env.storage()
            .persistent()
            .set(&DataKey::Quest(quest_id), &quest);

        QuestCreated {
            employer: admin,
            quest_id,
            reward_amount,
        }
        .publish(&env);

        quest_id
    }

    /// Returns the [`Quest`] for the given ID, or `None` if not found.
    ///
    /// # Arguments
    ///
    /// * `quest_id` — The quest ID to look up.
    ///
    /// # Returns
    ///
    /// `Some(Quest)` if found, `None` otherwise — callers can branch on
    /// presence without panicking.
    pub fn get_quest(env: Env, quest_id: u32) -> Option<Quest> {
        env.storage().persistent().get(&DataKey::Quest(quest_id))
    }

    /// Stores a learner's proof for a Build quest.
    ///
    /// The quest must be active and of [`QuestType::Build`]. Each
    /// `(learner, quest_id)` pair may only have one submission; re-submission panics.
    ///
    /// # Arguments
    ///
    /// * `learner` — Address submitting the proof (required auth).
    /// * `quest_id` — ID of the Build quest.
    /// * `proof_hash` — 32-byte IPFS or on-chain hash of the proof artifact.
    ///
    /// # Panics
    ///
    /// * `"Quest not found"` — if the quest ID does not exist.
    /// * `"Quest is not active"` — if the quest has been deactivated.
    /// * `"Only Build quests accept submissions"` — if quest type is `Explore`.
    /// * `"Submission already exists"` — if a submission already exists for this pair.
    pub fn submit_proof(env: Env, learner: Address, quest_id: u32, proof_hash: BytesN<32>) {
        learner.require_auth();

        let quest: Quest = env
            .storage()
            .persistent()
            .get(&DataKey::Quest(quest_id))
            .expect("Quest not found");
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

        ProofSubmitted {
            learner,
            quest_id,
            proof_hash,
        }
        .publish(&env);
    }

    /// Returns the [`Submission`] for a given learner and quest, or `None`.
    ///
    /// `None` means the learner has not yet submitted proof for `quest_id`.
    pub fn get_submission(env: Env, learner: Address, quest_id: u32) -> Option<Submission> {
        env.storage()
            .persistent()
            .get(&DataKey::Submission(learner, quest_id))
    }

    /// Approves or rejects a single learner submission for a Build quest.
    ///
    /// On approval the reward is split: 15 % platform fee goes to the RewardPool
    /// address and the remainder to the learner, scaled by the learner's
    /// [`StakeVault`] multiplier (capped at the available post-fee balance).
    ///
    /// # Arguments
    ///
    /// * `employer` — Must equal `quest.employer` (required auth).
    /// * `learner` — Address whose submission is being reviewed.
    /// * `quest_id` — ID of the quest.
    /// * `approve` — `true` to approve and pay; `false` to reject.
    ///
    /// # Panics
    ///
    /// * `"Contract is paused"` — if the pause flag is set.
    /// * `"Quest not found"` — if the quest ID does not exist.
    /// * `"Only the quest employer can review submissions"` — wrong caller.
    /// * `"Submission not found"` — if no submission exists.
    /// * `"Submission is not pending review"` — if already decided.
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

        let quest: Quest = env
            .storage()
            .persistent()
            .get(&DataKey::Quest(quest_id))
            .expect("Quest not found");
        if quest.employer != employer {
            panic!("Only the quest employer can review submissions");
        }

        let submission_key = DataKey::Submission(learner.clone(), quest_id);
        let mut submission: Submission = env
            .storage()
            .persistent()
            .get(&submission_key)
            .expect("Submission not found");
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

            // Multiplier from StakeVault (BPS): 100 = 1.0×, 120 = 1.2×, 200 = 2.0×.
            // Boost capped to base_learner_amount since employers fund at face value.
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

        SubmissionReviewed {
            employer,
            learner,
            quest_id,
            approved: approve,
        }
        .publish(&env);
    }

    /// Cancels an active Build quest and returns the locked tokens to the employer.
    ///
    /// Marks the quest inactive and transfers `reward_amount` back to the employer.
    ///
    /// # Arguments
    ///
    /// * `employer` — Must equal `quest.employer` (required auth).
    /// * `quest_id` — ID of the quest to cancel.
    ///
    /// # Panics
    ///
    /// * `"Quest not found"` — if the quest ID does not exist.
    /// * `"Unauthorized"` — if `employer ≠ quest.employer`.
    /// * `"Quest already inactive"` — if the quest is already cancelled.
    pub fn refund_quest(env: Env, employer: Address, quest_id: u32) {
        employer.require_auth();

        let mut quest: Quest = env
            .storage()
            .persistent()
            .get(&DataKey::Quest(quest_id))
            .expect("Quest not found");

        if quest.employer != employer {
            panic!("Unauthorized");
        }
        if !quest.active {
            panic!("Quest already inactive");
        }

        quest.active = false;
        env.storage()
            .persistent()
            .set(&DataKey::Quest(quest_id), &quest);

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

    /// Approves all submissions in `learners` for a single quest in one transaction.
    ///
    /// Each submission must be `Pending`; the function panics on the first
    /// non-pending entry. Emits one [`SubmissionReviewed`] per learner and a
    /// single [`BatchReviewed`] summary at the end.
    ///
    /// # Arguments
    ///
    /// * `employer` — Must equal `quest.employer` (required auth).
    /// * `quest_id` — ID of the quest.
    /// * `learners` — Ordered list of learner addresses to approve.
    ///
    /// # Panics
    ///
    /// * `"Contract is paused"` — if the pause flag is set.
    /// * `"Quest not found"` — if the quest ID does not exist.
    /// * `"Only the quest employer can review submissions"` — wrong caller.
    /// * `"Submission not found"` — if a learner has no submission.
    /// * `"Submission is not pending review"` — if already decided.
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

        let quest: Quest = env
            .storage()
            .persistent()
            .get(&DataKey::Quest(quest_id))
            .expect("Quest not found");
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

            if submission.status != SubmissionStatus::Pending {
                panic!("Submission is not pending review");
            }

            let fee = (quest.reward_amount * 15) / 100;
            let learner_amount = quest.reward_amount - fee;

            token_client.transfer(&env.current_contract_address(), &reward_pool, &fee);
            token_client.transfer(&env.current_contract_address(), &learner, &learner_amount);

            submission.status = SubmissionStatus::Approved;
            env.storage().persistent().set(&submission_key, &submission);

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

    /// Upgrades the contract WASM to a new hash. Only callable by the protocol admin.
    ///
    /// Replaces the QuestEngine WASM on the Soroban host and emits
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

    /// Verifies an Explore quest completion and triggers a RewardPool payout.
    ///
    /// Admin-only confirmation that a learner completed an off-chain action.
    /// Triggers a cross-contract `distribute_reward` call; the QuestEngine
    /// must be whitelisted as an approved spender on the RewardPool.
    ///
    /// # Arguments
    ///
    /// * `admin` — Must equal the stored admin (required auth).
    /// * `learner` — Address to receive the reward.
    /// * `quest_id` — ID of the Explore quest being verified.
    ///
    /// # Panics
    ///
    /// * `"Not initialized"` — if the contract has not been initialized.
    /// * `"Unauthorized"` — if `admin ≠ stored admin`.
    /// * `"Quest not found"` — if the quest ID does not exist.
    /// * `"Not an Explore quest"` — if the quest type is `Build`.
    pub fn verify_explore_quest(env: Env, admin: Address, learner: Address, quest_id: u32) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        assert!(admin == stored_admin, "Unauthorized");

        let quest: Quest = env
            .storage()
            .persistent()
            .get(&DataKey::Quest(quest_id))
            .expect("Quest not found");

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
