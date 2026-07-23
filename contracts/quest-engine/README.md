# Quest Engine Contract

Build quests (employer-funded, peer-reviewed) and Explore quests (admin-verified, pool-funded).

## Functions

- `initialize(admin, token, reward_pool, stake_vault)` — one-time deploy-time setup.
- `set_pause(admin, status)` — admin-only circuit breaker.
- `create_build_quest(employer, reward_amount, metadata_hash)` — escrow.
- `create_explore_quest(admin, reward_amount, metadata_hash)` — pool-backed.
- `submit_proof(learner, quest_id, proof_hash)` — single per learner per quest.
- `review_submission(employer, learner, quest_id, approve)` — single review with multiplier.
- `batch_review_submissions(employer, quest_id, learners)` — bulk review.
- `refund_quest(employer, quest_id)` — employer-only cancel + refund.
- `verify_explore_quest(admin, learner, quest_id)` — admin-only pool payout.
- `get_quest(quest_id)` / `get_submission(learner, quest_id)` — view accessors.
- `upgrade_contract(admin, new_wasm_hash)` — admin-only WASM upgrade.
- `set_reward_pool_address(admin, new_address)` / `set_stake_vault_address(admin, new_address)` — admin-only single-step wiring setters (kept for the initial bootstrap).

## Two-Step Admin & Wiring Transfer (Issue #20)

`Admin`, `RewardPool`, and `StakeVault` are all rotated through
`propose → accept`. The single-step setters remain for the initial
wiring at deploy time; later rotations must go through the two-step
flow:

| Role | Propose | Accept | Cancel |
| ---- | ------- | ------ | ------ |
| Admin | `propose_new_admin(current_admin, proposed)` | `accept_admin_ownership(acceptor)` | `cancel_admin_transfer(caller)` |
| RewardPool | `propose_new_reward_pool_address(current_admin, proposed)` | `accept_reward_pool_address(acceptor)` | `cancel_reward_pool_transfer(caller)` |
| StakeVault | `propose_new_stake_vault_address(current_admin, proposed)` | `accept_stake_vault_address(acceptor)` | `cancel_stake_vault_transfer(caller)` |

Cancellable by the current admin OR by the (typo'd) proposed address.
Soft timelock: accept is immediate. Events are the shared
`TransferProposed` / `TransferAccepted` / `TransferCancelled` from
`contracts/common::two_step`.

