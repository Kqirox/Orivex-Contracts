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
